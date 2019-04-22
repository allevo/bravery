use crate::http::Http;
use crate::http_protocol::{Parsed, HttpProtocolParser};
use tokio::io;
use bytes::BytesMut;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct Request<T: Clone> {
    pub parsed: Parsed,
    pub body: Vec<u8>,
    pub context: T,
    pub logger: slog::Logger
}

static REQUEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

impl<C: Clone> Request<C> {
    pub fn decode(http: &Http<C>, mut buf: &mut BytesMut) -> io::Result<Option<Request<C>>> {
        if buf.is_empty() {
            return Ok(None);
        }
        let parser = HttpProtocolParser {};

        let parsed = parser.parse(&mut buf);

        let body = match parsed.content_length {
            Some((start, end)) => {
                let content_length = &parsed.http_message_headers[start..end];
                let content_length = convert_u8_to_usize(content_length);
                let body = buf.split_to(content_length);
                body.to_vec()
            },
            None => Vec::new()
        };

        let logger = slog::Logger::new(&http.logger, o!(
            "reqId" => REQUEST_COUNTER.fetch_add(1, Ordering::SeqCst)
        ));

        Ok(Some(Request {
            body,
            context: http.context.clone(),
            logger,
            parsed
        }))
    }

    pub fn body_as<'a, T>(&'a self) -> serde_json::Result<T>
        where T: serde::de::Deserialize<'a>
    {
        serde_json::from_slice(&self.body)
    }

    pub fn query_string_as<'a, T>(&'a self) -> Result<T, serde::de::value::Error>
        where T: serde::de::Deserialize<'a>
    {
        serde_urlencoded::from_str(self.get_query_string())
    }

    pub fn get_method(&self) -> &str {
        let method = &self.parsed.http_message_headers[self.parsed.method_indexes.0..self.parsed.method_indexes.1];
        unsafe { std::str::from_utf8_unchecked(method) }
    }

    pub fn get_path(&self) -> &str {
        let path = &self.parsed.http_message_headers[self.parsed.path_indexes.0..self.parsed.path_indexes.1];
        unsafe { std::str::from_utf8_unchecked(path) }
    }

    pub fn get_query_string(&self) -> &str {
        let query_string = &self.parsed.http_message_headers[self.parsed.query_string_indexes.0..self.parsed.query_string_indexes.1];
        unsafe { std::str::from_utf8_unchecked(query_string) }
    }
}

fn convert_u8_to_usize(buff: &[u8]) -> usize {
    let zero_char: i16 = i16::from(b'0');
    let len = buff.len();
    let mut tot: usize = 0;
    for i in (0..len).rev() {
        let num = i16::from(buff[i]) - zero_char;
        if num < 0 || num > 9 {
            break;
        }
        tot += num as usize * 10usize.pow((len - 1 - i) as u32);
    }
    tot
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convert_u8_to_usize_1() {
        let input = [ 0, 0, 0, b'4' ];
        assert_eq!(convert_u8_to_usize(&input), 4);
    }

    #[test]
    fn convert_u8_to_usize_2() {
        let input = [ 0, 0, b'4', b'2' ];
        assert_eq!(convert_u8_to_usize(&input), 42);
    }

    #[test]
    fn convert_u8_to_usize_3() {
        let input = [ 0, b' ', b'4', b'2' ];
        assert_eq!(convert_u8_to_usize(&input), 42);
    }
}
