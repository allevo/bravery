use tokio_io::codec::{Encoder, Decoder};
use tokio::io;
use bytes::BytesMut;

use crate::request::Request;
use crate::response::Response;

pub struct Http<T> {
    pub with_headers: bool,
    pub with_query_string: bool,
    pub context: T,
    pub logger: slog::Logger
}

impl<T: Clone> Decoder for Http<T> {
    type Item = Request<T>;
    type Error = io::Error;

    fn decode<'b>(&mut self, buf: &'b mut BytesMut) -> io::Result<Option<Request<T>>> {
        Request::decode(self, buf)
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

    use serde::{Deserialize};

    use sloggers::Build;
    use sloggers::terminal::{TerminalLoggerBuilder, Destination};
    use sloggers::types::Severity;

    fn get_logger () -> slog::Logger {
        let mut builder = TerminalLoggerBuilder::new();
        builder.level(Severity::Debug);
        builder.destination(Destination::Stdout);
        builder.build().unwrap()
    }

    #[test]
    fn http_decode_get() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET / HTTP/1.1\r\nHost: localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let mut http = Http {
            with_headers: false,
            with_query_string: false,
            context: 0,
            logger: get_logger(),
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.get_path(), "/");
        assert_eq!(request.get_method(), "GET");
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
            logger: get_logger(),
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.get_path(), "/");
        assert_eq!(request.get_method(), "POST");
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
            logger: get_logger(),
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.get_path(), "/");
        assert_eq!(request.get_method(), "GET");
        assert_eq!(request.body, b"");
        assert_eq!(request.context, 0);

        let empty_vec: Vec<u8> = Vec::new();
        assert_eq!(input.to_vec(), empty_vec);
    }

    #[derive(Debug, Deserialize)]
    struct MyParams<'a> {
        key1: &'a str,
        key2: &'a str,
    }

    #[test]
    fn http_decode_get_with_query_parameters() {
        let mut input = BytesMut::new();
        input.extend_from_slice(b"GET /?key1=value1&key2=value2 HTTP/1.1\r\nHost: localhost:8880\r\nUser-Agent: curl/7.54.0\r\nAccept: */*\r\n\r\n");

        let mut http = Http {
            with_headers: false,
            with_query_string: true,
            context: 0,
            logger: get_logger(),
        };
        let request = http.decode(&mut input);

        assert!(request.is_ok());
        let request = request.unwrap().unwrap();

        assert_eq!(request.get_path(), "/");
        assert_eq!(request.get_method(), "GET");
        let decoded = request.query_string_as::<MyParams>().unwrap();
        assert_eq!(decoded.key1, "value1");
        assert_eq!(decoded.key2, "value2");
        assert_eq!(request.body, b"");
        assert_eq!(request.context, 0);

        let empty_vec: Vec<u8> = Vec::new();
        assert_eq!(input.to_vec(), empty_vec);
    }
}
