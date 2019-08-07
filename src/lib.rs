extern crate tokio;

use std::sync::{Arc};
use regex::Regex;
use core::hash::Hasher;
use core::hash::Hash;
use std::collections::HashMap;
use tokio::io;
use tokio::net::TcpListener;
use tokio::prelude::*;

use bravery_router::{create_root_node, find, add, optimize, Node};

use std::net::SocketAddr;
use tokio_codec::Framed;
use std::str;

#[macro_use]
extern crate slog;
extern crate sloggers;

#[macro_use]
extern crate serde_derive;

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

#[derive(Serialize)]
pub struct HttpError {
    pub status_code: u16,
    pub error_message: String,
    pub details: String,
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
    get_router: Node<usize>,
    get_handlers: Vec<Box<dyn Handler<T> + Send + Sync>>,
    post_router: Node<usize>,
    post_handlers: Vec<Box<dyn Handler<T> + Send + Sync>>,
    context: T,
    logger: slog::Logger,
    not_found: Box<dyn Handler<T> + Send + Sync>,
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
            get_router: create_root_node(),
            get_handlers: vec![],
            post_router: create_root_node(),
            post_handlers: vec![],
            context: EmptyState {},
            logger: get_logger(),
            not_found: Box::new(HandlerFor404 {}),
        }
    }
}

impl<T: 'static + Clone + Send + Sync> App<T> {
    pub fn new_with_state(context: T) -> App<T> {
        App {
            get_router: create_root_node(),
            get_handlers: vec![],
            post_router: create_root_node(),
            post_handlers: vec![],
            context,
            logger: get_logger(),
            not_found: Box::new(HandlerFor404 {}),
        }
    }

    pub fn get(self: &mut App<T>, path: &str, handler: Box<dyn Handler<T> + Send + Sync>) {
        add(&mut self.get_router, path, self.get_handlers.len());
        self.get_handlers.push(handler);
    }

    pub fn post(self: &mut App<T>, path: &str, handler: Box<dyn Handler<T> + Send + Sync>) {
        add(&mut self.post_router, path, self.post_handlers.len());
        self.post_handlers.push(handler);
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

    pub fn run(mut self: App<T>, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let socket = TcpListener::bind(&addr)?;
        println!("Listening on: {}", addr);

        self.post_router = optimize(self.post_router);
        self.get_router = optimize(self.get_router);

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
    let (router, handlers) = match method.as_ref() {
        "GET" => (&app.get_router, &app.get_handlers),
        "POST" => (&app.post_router, &app.post_handlers),
        _ => unimplemented!(),
    };

    let state_found = find(router, path);

    let func = match state_found.value {
        None => &app.not_found,
        Some(f) => handlers.get(*f).unwrap()
    };

    future::ok::<Response, io::Error>(func.invoke(request).or_else(|error: HttpError| {
        let fallback: String = "Unable to serialize".to_owned();
        let val: Result<String, _> = serde_json::to_string(&error);

        let body = if val.is_ok() { val.unwrap() } else { fallback };

        Ok::<Response, io::Error>(Response {
            status_code: error.status_code,
            content_type: Some("text/html".to_owned()),
            body,
            headers: HashMap::new()
        })
    }).unwrap())
}

pub fn error_500<E>(s: &'static str) -> impl Fn(E) -> HttpError {
    move |_e: E| -> HttpError {
        HttpError {
            status_code: 500,
            error_message: s.to_owned(),
            details: "".to_owned(),
        }
    }
}

use std::string::ToString;
pub fn error_400<Error: ToString>(s: &'static str) -> impl Fn(Error) -> HttpError {
    move |e: Error| -> HttpError {
        HttpError {
            status_code: 400,
            error_message: s.to_owned(),
            details: e.to_string(),
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
