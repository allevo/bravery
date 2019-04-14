use tokio_io::codec::{Encoder, Decoder};
use tokio::io;
use bytes::BytesMut;
use httparse::Status::{Complete, Partial};
use std::collections::HashMap;

use crate::request::Request;
use crate::response::Response;

pub struct Http<T> {
    pub with_headers: bool,
    pub with_query_string: bool,
    pub context: T
}

impl<T: Clone> Decoder for Http<T> {
    type Item = Request<T>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Request<T>>> {
        let mut headers = [httparse::EMPTY_HEADER; 16];
        let mut req = httparse::Request::new(&mut headers);
        let headers = req.parse(buf).unwrap();

        let header_lenght = match headers {
            Complete(hl) => hl,
            Partial => 0
        };
        if header_lenght == 0 {
            return Ok(None);
        }

        let method = req.method.unwrap();
        let url = req.path.unwrap();

        let index = url.find('?').or_else(|| Some(url.len())).unwrap();

        let path = &url[..index];
        let params = if self.with_query_string && index < url.len() { &url[(index + 1)..] } else { "" };

        let with_headers = self.with_headers;

        let content_type_header_name = "content-type";
        let content_length_header_name = "content-length";

        let mut content_length = 0;
        let mut content_type = None;
        let mut c: u8 = 0;
        let mut headers: HashMap<String, String> = HashMap::new();
        for header in req.headers.iter() {
            let header_name = header.name.to_string().to_lowercase();
            let header_value = String::from_utf8_lossy(header.value).to_string();

            if header_name == content_type_header_name {
                content_type = Some(header_value.to_string());
                c += 1;
            } else if header_name == content_length_header_name {
                content_length = header_value.parse::<usize>().unwrap();
                c += 1;
            }

            if !with_headers && c == 2 {
                break;
            }

            if with_headers {
                headers.insert(header_name, header_value);
            }
        }

        let request = Request {
            method: method.to_string(),
            path: path.to_string(),
            params: params.to_string(),
            headers,
            content_type,
            content_length,
            header_lenght,
            body: buf.split_to(header_lenght + content_length)[header_lenght..].to_vec(),
            context: self.context.clone()
        };

        Ok(Some(request))
    }
}

impl<T: Clone> Encoder for Http<T> {
    type Item = Response;
    type Error = io::Error;


    fn encode(&mut self, response: Response, buf: &mut BytesMut) -> io::Result<()> {
        let body = response.body;
        let len = body.len().to_string();
        let output = "HTTP/1.1 ".to_owned() + &response.status_code.to_string()[..] + " OK"
            + "\r\n"
            + "Content-length:" + &len[..]
            + "\r\n"
            + "\r\n"
            + &body[..];
        buf.extend_from_slice(output.as_bytes());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_decode_get() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET / HTTP/1.1\r\nHost: localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let mut http = Http {
            with_headers: false,
            with_query_string: false,
            context: 0,
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.path, "/");
        assert_eq!(request.method, "GET");
        assert_eq!(request.content_length, 0);
        assert_eq!(request.content_type, None);
        assert_eq!(request.headers, HashMap::new());
        assert_eq!(request.body, b"");
        assert_eq!(request.context, 0);

        let empty_vec: Vec<u8> = Vec::new();
        assert_eq!(input.to_vec(), empty_vec);
    }

    #[test]
    fn http_decode_post() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"POST / HTTP/1.1\r\nHost: localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\nContent-type: application/json\r\nContent-Length: 16\r\n\r\n{\"message\":\"aa\"}");

        let mut http = Http {
            with_headers: false,
            with_query_string: false,
            context: 0,
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.path, "/");
        assert_eq!(request.method, "POST");
        assert_eq!(request.content_length, 16);
        assert_eq!(request.content_type, Some("application/json".to_string()));
        assert_eq!(request.headers, HashMap::new());
        assert_eq!(request.body, b"{\"message\":\"aa\"}");
        assert_eq!(request.context, 0);

        let empty_vec: Vec<u8> = Vec::new();
        assert_eq!(input.to_vec(), empty_vec);
    }

    #[test]
    fn http_decode_get_with_headers() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET / HTTP/1.1\r\nHost: localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let mut http = Http {
            with_headers: true,
            with_query_string: false,
            context: 0,
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.path, "/");
        assert_eq!(request.method, "GET");
        assert_eq!(request.content_length, 0);
        assert_eq!(request.content_type, None);
        assert_eq!(request.headers, [
            ("accept".to_string(), "*/*".to_string()),
            ("host".to_string(), "localhost:8880".to_string()),
            ("user-agent".to_string(), "curl/7.54.0".to_string())
            ].iter().cloned().collect());
        assert_eq!(request.body, b"");
        assert_eq!(request.context, 0);

        let empty_vec: Vec<u8> = Vec::new();
        assert_eq!(input.to_vec(), empty_vec);
    }

    #[test]
    fn http_decode_get_with_query_parameters() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET /?key1=value1&key2=value2 HTTP/1.1\r\nHost: localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let mut http = Http {
            with_headers: false,
            with_query_string: true,
            context: 0,
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.path, "/");
        assert_eq!(request.method, "GET");
        assert_eq!(request.params, "key1=value1&key2=value2");
        assert_eq!(request.content_length, 0);
        assert_eq!(request.content_type, None);
        assert_eq!(request.headers, HashMap::new());
        assert_eq!(request.body, b"");
        assert_eq!(request.context, 0);

        let empty_vec: Vec<u8> = Vec::new();
        assert_eq!(input.to_vec(), empty_vec);
    }
}
