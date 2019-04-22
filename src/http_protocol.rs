use bytes::{BytesMut};

#[derive(Debug)]
pub struct Parsed {
    pub method_indexes: (usize, usize),
    pub path_indexes: (usize, usize),
    pub query_string_indexes: (usize, usize),
    pub content_type: Option<(usize, usize)>,
    pub content_length: Option<(usize, usize)>,
    pub http_message_headers: BytesMut,
}

pub struct HttpProtocolParser {}

impl HttpProtocolParser {
    pub fn parse(&self, buff: &mut BytesMut) -> Parsed {
        let mut index = None;
        for i in 0..buff.len() {
            if buff[i] == b'\r' && buff[i + 1] == b'\n' && buff[i + 2] == b'\r' && buff[i + 3] == b'\n' {
                index = Some(i);
                break;
            }
        }
        let index = index.unwrap();

        let http_message_headers = buff.split_to(index + 4);

        let mut lines = http_message_headers.split(|x| *x == 10 as u8);

        let start_line = lines.next().unwrap();

        let mut method_indexes = (0 as usize, 0 as usize);
        method_indexes.1 = if start_line[3] == b' ' { 3 } // Fast path for GET and PUT
            else if start_line[4] == b' ' { 4 } // Fast path for POST
            else if start_line[5] == b' ' { 5 } // Fast path for PATCH
            else if start_line[6] == b' ' { 6 } // Fast path for DELETE
            else { panic!("not implemented yet"); }; // TODO: implement me

        let mut path_indexes = (method_indexes.1 + 1, method_indexes.1 + 1);
        let mut query_string_indexes = (0 as usize, 0  as usize);
        let mut has_query_string = false;
        let iter = start_line[path_indexes.0..].iter().enumerate();
        for (i, c) in iter {
            if *c == b'?' {
                path_indexes.1 = path_indexes.0 + i;
                query_string_indexes.0 = path_indexes.0 + i + 1;
                has_query_string = true;
                continue;
            }
            if *c == b' ' {
                if has_query_string {
                    query_string_indexes.1 = path_indexes.0 + i;
                } else {
                    path_indexes.1 = path_indexes.0 + i;
                    query_string_indexes = (path_indexes.0 + i, path_indexes.0 + i);
                }
                break
            }
        }

        let mut content_length = None;
        let mut content_type = None;
        for i in 0..http_message_headers.len() {
            if true &&
                http_message_headers[i] == b'C' &&
                http_message_headers[i + 1] == b'o' &&
                http_message_headers[i + 2] == b'n' &&
                http_message_headers[i + 3] == b't' &&
                http_message_headers[i + 4] == b'e' &&
                http_message_headers[i + 5] == b'n' &&
                http_message_headers[i + 6] == b't' &&
                http_message_headers[i + 7] == b'-' &&
                http_message_headers[i + 8] == b'L' &&
                http_message_headers[i + 9] == b'e' &&
                http_message_headers[i + 10] == b'n' &&
                http_message_headers[i + 11] == b'g' &&
                http_message_headers[i + 12] == b't' &&
                http_message_headers[i + 13] == b'h' &&
                http_message_headers[i + 14] == b':'
            {
                for j in i..http_message_headers.len() {
                    if http_message_headers[j] == b'\r' {
                        content_length = Some((i, j));
                        break;
                    }
                }
                break;
            }

            if true &&
                http_message_headers[i] == b'C' &&
                http_message_headers[i + 1] == b'o' &&
                http_message_headers[i + 2] == b'n' &&
                http_message_headers[i + 3] == b't' &&
                http_message_headers[i + 4] == b'e' &&
                http_message_headers[i + 5] == b'n' &&
                http_message_headers[i + 6] == b't' &&
                http_message_headers[i + 7] == b'-' &&
                http_message_headers[i + 8] == b'T' &&
                http_message_headers[i + 9] == b'y' &&
                http_message_headers[i + 10] == b'p' &&
                http_message_headers[i + 11] == b'e' &&
                http_message_headers[i + 12] == b':'
            {
                for j in i..http_message_headers.len() {
                    if http_message_headers[j] == b'\r' {
                        content_type = Some((i, j));
                        break;
                    }
                }
                break;
            }
        }

        Parsed {
            method_indexes,
            path_indexes,
            query_string_indexes,
            content_type,
            content_length,
            http_message_headers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_get_simple() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET / HTTP/1.1\r\nHost:localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut input);

        assert_eq!(&parsed.http_message_headers[parsed.method_indexes.0..parsed.method_indexes.1], b"GET");
        assert_eq!(&parsed.http_message_headers[parsed.path_indexes.0..parsed.path_indexes.1], b"/");
        assert_eq!(&parsed.http_message_headers[parsed.query_string_indexes.0..parsed.query_string_indexes.1], b"");

        assert_eq!(input.len(), 0);
    }

    #[test]
    fn http_get_with_query_string() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET /?a=b HTTP/1.1\r\nHost:localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut input);

        assert_eq!(&parsed.http_message_headers[parsed.method_indexes.0..parsed.method_indexes.1], b"GET");
        assert_eq!(&parsed.http_message_headers[parsed.path_indexes.0..parsed.path_indexes.1], b"/");
        assert_eq!(&parsed.http_message_headers[parsed.query_string_indexes.0..parsed.query_string_indexes.1], b"a=b");

        assert_eq!(input.len(), 0);
    }

    #[test]
    fn http_post_simple() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"POST / HTTP/1.1\r\nHost:localhost:8880\r\nContent-Length: 0\r\nAccept: */*\r\n\r\n");

        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut input);

        assert_eq!(&parsed.http_message_headers[parsed.method_indexes.0..parsed.method_indexes.1], b"POST");
        assert_eq!(&parsed.http_message_headers[parsed.path_indexes.0..parsed.path_indexes.1], b"/");
        assert_eq!(&parsed.http_message_headers[parsed.query_string_indexes.0..parsed.query_string_indexes.1], b"");

        assert_eq!(input.len(), 0);
    }

    #[test]
    fn http_post_with_query_string() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"POST /?a=b HTTP/1.1\r\nHost:localhost:8880\r\nContent-Length: 0\r\nAccept: */*\r\n\r\n");

        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut input);

        assert_eq!(&parsed.http_message_headers[parsed.method_indexes.0..parsed.method_indexes.1], b"POST");
        assert_eq!(&parsed.http_message_headers[parsed.path_indexes.0..parsed.path_indexes.1], b"/");
        assert_eq!(&parsed.http_message_headers[parsed.query_string_indexes.0..parsed.query_string_indexes.1], b"a=b");

        assert_eq!(input.len(), 0);
    }

    #[test]
    fn http_post_with_body() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"POST /?a=b HTTP/1.1\r\nHost:localhost:8880\r\nContent-Length: 4\r\nAccept: */*\r\n\r\naaaa");

        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut input);

        assert_eq!(&parsed.http_message_headers[parsed.method_indexes.0..parsed.method_indexes.1], b"POST");
        assert_eq!(&parsed.http_message_headers[parsed.path_indexes.0..parsed.path_indexes.1], b"/");
        assert_eq!(&parsed.http_message_headers[parsed.query_string_indexes.0..parsed.query_string_indexes.1], b"a=b");

        assert_eq!(input.len(), 4);
    }

    #[test]
    fn http_delete() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"DELETE / HTTP/1.1\r\n\r\n");

        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut input);

        assert_eq!(&parsed.http_message_headers[parsed.method_indexes.0..parsed.method_indexes.1], b"DELETE");
        assert_eq!(&parsed.http_message_headers[parsed.path_indexes.0..parsed.path_indexes.1], b"/");
        assert_eq!(&parsed.http_message_headers[parsed.query_string_indexes.0..parsed.query_string_indexes.1], b"");

        assert_eq!(input.len(), 0);
    }
}
