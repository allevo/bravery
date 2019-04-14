extern crate tokio;

use std::sync::{Arc};
use regex::Regex;
use core::hash::Hasher;
use core::hash::Hash;
use std::collections::HashMap;
use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

use std::net::SocketAddr;
use tokio_io::codec::{Encoder, Decoder};
use bytes::BytesMut;
use tokio_codec::Framed;
use httparse::Status::{Complete, Partial};

struct Http<T> {
    with_headers: bool,
    with_query_string: bool,
    context: T
}
pub struct Request<T: Clone> {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub content_type: Option<String>,
    pub content_length: usize,
    pub header_lenght: usize,
    body: Vec<u8>,
    pub context: T
}
pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub content_type: Option<String>,
    pub body: String
}

use std::str;

impl<C: Clone> Request<C> {
    pub fn body_as<'a, T>(&'a self) -> serde_json::Result<T>
        where T: serde::de::Deserialize<'a>
    {
        serde_json::from_slice(&self.body)
    }
}

impl<T: Clone> Decoder for Http<T> {
    type Item = Request<T>;
    type Error = io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> io::Result<Option<Request<T>>> {
        // println!("decode {:?}", buf);

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

        let method = req.method;
        let path = req.path;

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

        if self.with_query_string {
            // TODO
        }

        let request = Request {
            method: method.unwrap().to_string(),
            path: path.unwrap().to_string(),
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

#[derive(Debug)]
struct MatchedRouter {
    s: String,
    regex: Regex,
    method: String,
}

impl PartialEq for MatchedRouter {
    fn eq(&self, other: &MatchedRouter) -> bool {
        self.s == other.s && self.method == other.method
    }
}
impl Eq for MatchedRouter {}

impl Hash for MatchedRouter {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.s.hash(state);
        self.method.hash(state);
    }
}

pub struct HttpError {
    pub error_message: String
}

impl std::fmt::Debug for HttpError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "HTTPError"
        )
    }
}

pub trait Handler<T: Clone> {
    fn invoke(&self, req: Request<T>) -> Result<Response, HttpError>;
}

pub struct App<T> {
    router: HashMap<MatchedRouter, Box<Handler<T> + Send + Sync>>,
    context: T
}

impl<T: 'static +  Clone + Send + Sync> App<T> {
    pub fn new_with_state(context: T) -> App<T> {
        App {
            router: HashMap::new(),
            context
        }
    }

    pub fn get(self: &mut App<T>, path: &str, handler: Box<Handler<T> + Send + Sync>) {
        self.router.insert(MatchedRouter {
            method: "GET".to_string(),
            s: path.to_string(),
            regex: Regex::new(&path.to_string()).unwrap(),
        }, handler);
    }

    pub fn post(self: &mut App<T>, path: &str, handler: Box<Handler<T> + Send + Sync>) {
        self.router.insert(MatchedRouter {
            method: "POST".to_string(),
            s: path.to_string(),
            regex: Regex::new(&path.to_string()).unwrap(),
        }, handler);
    }

    pub fn run(self: App<T>, addr: SocketAddr) -> Result<(), Box<std::error::Error>> {
        let socket = TcpListener::bind(&addr)?;
        println!("Listening on: {}", addr);

        let app = Arc::new(self);

        let done = socket
            .incoming()
            .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            .for_each(move |socket| {
                let http: Http<T> = Http {
                    with_headers: false,
                    with_query_string: false,
                    context: app.context.clone()
                };
                let framed = Framed::new(socket, http);

                let (tx, rx) = framed.split();

                let app = app.clone();

                let task = tx.send_all(rx.and_then(move |request: Request<T>| {
                        resolve(app.clone(), request)
                    }))
                    .then(|_| future::ok(()));

                tokio::spawn(task)
            });

        tokio::run(done);

        Ok(())
    }
}

fn resolve<T: Clone>(app: Arc<App<T>>, request: Request<T>) -> impl Future<Item=Response, Error=io::Error> + Send {
    let method = &request.method;
    let path = &request.path;

    for matched_router in (*app).router.keys() {
        if matched_router.method != *method || matched_router.s != *path || !matched_router.regex.is_match(path) {
            continue
        }
        let func = &(*app).router[matched_router];

        return future::ok(func.invoke(request).unwrap());
    }

    // 404
    future::ok(Response {
        status_code: 404,
        content_type: Some("text/html".to_string()),
        body: "".to_string(),
        headers: HashMap::new()
    })
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

pub fn error_500<E>(s: &'static str) -> impl Fn(E) -> HttpError {
    move |_e: E| -> HttpError {
        HttpError {
            error_message: s.to_string()
        }
    }
}

#[derive(Clone)]
pub struct EmptyState;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
