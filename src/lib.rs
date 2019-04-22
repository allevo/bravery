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
use tokio_codec::Framed;
use std::str;

#[macro_use]
extern crate slog;
extern crate sloggers;

use sloggers::Build;
use sloggers::terminal::{TerminalLoggerBuilder, Destination};
use sloggers::types::Severity;

pub mod request;
pub mod response;
pub mod http;

pub use self::request::Request;
pub use self::response::Response;
pub use self::http::Http;

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
    status_code: u16,
    error_message: String
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
    context: T,
    logger: slog::Logger
}

fn get_logger () -> slog::Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stdout);
    builder.build().unwrap()
}

impl Default for App<EmptyState> {
    fn default() -> App<EmptyState> {
        App {
            router: HashMap::new(),
            context: EmptyState {},
            logger: get_logger(),
        }
    }
}

impl<T: 'static +  Clone + Send + Sync> App<T> {
    pub fn new_with_state(context: T) -> App<T> {
        App {
            router: HashMap::new(),
            context,
            logger: get_logger(),
        }
    }

    pub fn get(self: &mut App<T>, path: &str, handler: Box<Handler<T> + Send + Sync>) {
        self.router.insert(MatchedRouter {
            method: "GET".to_owned(),
            s: path.to_owned(),
            regex: Regex::new(&path.to_owned()).unwrap(),
        }, handler);
    }

    pub fn post(self: &mut App<T>, path: &str, handler: Box<Handler<T> + Send + Sync>) {
        self.router.insert(MatchedRouter {
            method: "POST".to_owned(),
            s: path.to_owned(),
            regex: Regex::new(&path.to_owned()).unwrap(),
        }, handler);
    }

    pub fn inject(self: &App<T>, request: Request<T>) -> Response {
        resolve(self, request).wait().unwrap()
    }

    pub fn create_request(self: &App<T>, method: &str, path: &str, query_string: &str, body: Vec<u8>) -> Request<T> {
        Request {
            path: path.to_owned(),
            method: method.to_owned(),
            content_length: body.len(),
            content_type: None,
            header_lenght: 0,
            query_string: query_string.to_owned(),
            headers: HashMap::new(),
            body,
            context: self.context.clone(),
            logger: self.logger.clone()
        }
    }

    pub fn run(self: App<T>, addr: SocketAddr) -> Result<(), Box<std::error::Error>> {
        let socket = TcpListener::bind(&addr)?;
        println!("Listening on: {}", addr);

        let app = Arc::new(self);

        let done = socket
            .incoming()
            .map_err(|e| println!("failed to accept socket; error = {:?}", e))
            .for_each(move |socket| {
                // TODO: clone this
                let http: Http<T> = Http {
                    with_headers: false,
                    with_query_string: true,
                    context: app.context.clone(),
                    logger: app.logger.clone()
                };
                let framed = Framed::new(socket, http);

                let (tx, rx) = framed.split();

                let app = app.clone();

                let task = tx.send_all(rx.and_then(move |request: Request<T>| {
                        resolve(&*app, request)
                    }))
                    .then(|_| future::ok(()));

                tokio::spawn(task)
            });

        tokio::run(done);

        Ok(())
    }
}

struct HandlerFor404 {}
impl<T: Clone> Handler<T> for HandlerFor404 {
    fn invoke(&self, _req: Request<T>) -> Result<Response, HttpError> {
        Ok(Response {
            status_code: 404,
            content_type: Some("text/html".to_owned()),
            body: "404 Handler".to_owned(),
            headers: HashMap::new()
        })
    }
}

fn resolve<T: Clone>(app: &App<T>, request: Request<T>) -> impl Future<Item=Response, Error=io::Error> + Send {
    let method = &request.method;
    let path = &request.path;
    let router = &app.router;

    let not_found: Box<Handler<T> + Send + Sync> = Box::new(HandlerFor404 {});
    let m = router.iter().find(|(matched_router, _value)| {
        matched_router.method == *method && matched_router.s == *path && matched_router.regex.is_match(path)
    });

    let func = match m {
        None => &not_found,
        Some((_m, f)) => f
    };

    future::ok::<Response, io::Error>(func.invoke(request).or_else(|e: HttpError| {
        Ok::<Response, io::Error>(Response {
            status_code: e.status_code,
            content_type: Some("text/html".to_owned()),
            body: e.error_message,
            headers: HashMap::new()
        })
    }).unwrap())
}

pub fn error_500<E>(s: &'static str) -> impl Fn(E) -> HttpError {
    move |_e: E| -> HttpError {
        HttpError {
            status_code: 500,
            error_message: s.to_owned()
        }
    }
}

pub fn error_400<E>(s: &'static str) -> impl Fn(E) -> HttpError {
    move |_e: E| -> HttpError {
        HttpError {
            status_code: 400,
            error_message: s.to_owned()
        }
    }
}

#[derive(Clone)]
pub struct EmptyState;

#[cfg(test)]
mod tests {
    use super::*;

    struct MyHandler {}
    impl<T: Clone> Handler<T> for MyHandler {
        fn invoke(&self, _req: Request<T>) -> Result<Response, HttpError> {
            Ok(Response {
                status_code: 200,
                content_type: Some("text/html".to_owned()),
                body: "MyHandler".to_owned(),
                headers: HashMap::new()
            })
        }
    }

    fn get_app<T: 'static>(t: T) -> App<T>
        where T: Send + Sync + Clone
    {
        let mut app = App::new_with_state(t);
        app.get("/", Box::new(MyHandler {}));
        app
    }

    #[test]
    fn dispatch_requests() {
        let app = get_app(0);

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 200);

        let request = app.create_request("GET", "/unknwon-path", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 404);
    }
}
