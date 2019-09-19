#![warn(rust_2018_idioms)]

extern crate tokio;
// extern crate tokio_signal;

use core::hash::Hash;
use core::hash::Hasher;
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;

use bravery_router::{add, create_root_node, find, optimize, Node};

use objekt;
use std::net::SocketAddr;
use std::str;
use tokio::prelude::*;
use tokio::{
    codec::Framed,
    net::{tcp::TcpStream, TcpListener},
};

#[macro_use]
extern crate slog;
extern crate sloggers;

#[macro_use]
extern crate serde_derive;

use sloggers::terminal::{Destination, TerminalLoggerBuilder};
use sloggers::types::Severity;
use sloggers::Build;

use futures::executor::block_on;

pub mod http;
pub mod request;
pub mod response;

pub use self::http::HttpCodec;
pub use self::request::Request;
pub use self::response::Response;

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
        write!(f, "HTTPError")
    }
}

pub trait Handler<T: Clone + Send + Sync>: objekt::Clone + Sync + Send {
    fn invoke(&self, req: Request<T>) -> Result<Response, HttpError>;
}
objekt::clone_trait_object!(<T: Clone + Send + Sync> Handler<T>);

async fn process_socket<T: Clone + Sync + Send + Unpin>(
    app: Arc<App<T>>,
    socket: TcpStream,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut framed = Framed::<TcpStream, HttpCodec<T>>::new(
        socket,
        HttpCodec {
            logger: app.logger.clone(),
            with_headers: true,
            with_query_string: true,
            context: app.context.clone(),
        },
    );

    while let Some(request) = framed.next().await {
        match request {
            Ok(request) => {
                let response = resolve(&app, request).await?;
                framed.send(response).await?;
            }
            Err(e) => return Err(e.into()),
        }
    }

    Ok(())
}

#[derive(Clone)]
pub struct App<T: 'static + Clone + Sync + Send> {
    get_router: Node<usize>,
    get_handlers: Vec<Box<dyn Handler<T>>>,
    post_router: Node<usize>,
    post_handlers: Vec<Box<dyn Handler<T>>>,
    logger: slog::Logger,
    context: T,
    not_found: Box<dyn Handler<T>>,
}

fn get_logger() -> slog::Logger {
    let mut builder = TerminalLoggerBuilder::new();
    builder.level(Severity::Debug);
    builder.destination(Destination::Stdout);
    builder.build().unwrap()
}

impl Default for App<EmptyState> {
    fn default() -> Self {
        App {
            get_router: create_root_node(),
            get_handlers: vec![],
            post_router: create_root_node(),
            post_handlers: vec![],
            logger: get_logger(),
            context: EmptyState {},
            not_found: Box::new(HandlerFor404 {}),
        }
    }
}

impl<T: Clone + Send + Sync + Unpin> App<T> {
    pub fn new_with_state(t: T) -> Self {
        App {
            get_router: create_root_node(),
            get_handlers: vec![],
            post_router: create_root_node(),
            post_handlers: vec![],
            logger: get_logger(),
            context: t,
            not_found: Box::new(HandlerFor404 {}),
        }
    }

    pub fn get(self: &mut App<T>, path: &str, handler: Box<dyn Handler<T>>) {
        add(&mut self.get_router, path, self.get_handlers.len());
        self.get_handlers.push(handler);
    }

    pub fn post(self: &mut App<T>, path: &str, handler: Box<dyn Handler<T>>) {
        add(&mut self.post_router, path, self.post_handlers.len());
        self.post_handlers.push(handler);
    }

    pub fn inject(self: &App<T>, request: Request<T>) -> Response {
        block_on(resolve(self, request)).unwrap()
    }

    pub fn create_request(
        self: &App<T>,
        method: &str,
        path: &str,
        query_string: &str,
        body: Vec<u8>,
    ) -> Request<T> {
        Request {
            path: path.to_owned(),
            method: method.to_owned(),
            content_length: body.len(),
            content_type: None,
            header_lenght: 0,
            query_string: query_string.to_owned(),
            headers: HashMap::new(),
            body,
            logger: self.logger.clone(),
            context: self.context.clone(),
        }
    }

    pub fn run(mut self: App<T>, addr: SocketAddr) -> Result<(), Box<dyn std::error::Error>> {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let mut incoming = TcpListener::bind(&addr);
            let mut incoming = incoming.await;
            let mut incoming = incoming?;
            let mut incoming = incoming.incoming();

            self.post_router = optimize(self.post_router);
            self.get_router = optimize(self.get_router);

            let app = Arc::new(self);

            while let Some(Ok(stream)) = incoming.next().await {
                let app = app.clone();
                tokio::spawn(async move {
                    if let Err(e) = process_socket(app, stream).await {
                        println!("failed to process connection; error = {}", e);
                    }
                });
            }

            Ok(())
        })
    }
}

#[derive(Clone)]
struct HandlerFor404 {}
impl<T: Clone + Sync + Send> Handler<T> for HandlerFor404 {
    fn invoke(&self, _req: Request<T>) -> Result<Response, HttpError> {
        Ok(Response {
            status_code: 404,
            content_type: Some("text/html".to_owned()),
            body: "404 Handler".to_owned().into_bytes(),
            headers: HashMap::new(),
        })
    }
}

use percent_encoding::percent_decode_str;

async fn resolve<T: Clone + Sync + Send + Unpin>(
    app: &App<T>,
    request: Request<T>,
) -> Result<Response, Box<dyn std::error::Error>> {
    let method = &request.method;
    let path = &request.path;
    let (router, handlers): (&Node<usize>, &Vec<Box<dyn Handler<T>>>) = match method.as_ref() {
        "GET" => (&app.get_router, &app.get_handlers),
        "POST" => (&app.post_router, &app.post_handlers),
        _ => unimplemented!(),
    };

    let path = percent_decode_str(path).decode_utf8_lossy();
    let state_found = find(router, &path);

    let func = match state_found.value {
        None => &app.not_found,
        Some(f) => handlers.get(*f).unwrap(),
    };

    func.invoke(request).or_else(|error: HttpError| {
        let fallback: Vec<u8> = "Unable to serialize".to_owned().into_bytes();
        let val: Result<Vec<u8>, _> = serde_json::to_vec(&error);

        let body = if let Ok(v) = val { v } else { fallback };

        Ok::<Response, Box<dyn std::error::Error>>(Response {
            status_code: error.status_code,
            content_type: Some("text/html".to_owned()),
            body,
            headers: HashMap::new(),
        })
    })
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

    #[derive(Clone)]
    struct MyHandler {}
    impl<T: Clone + Sync + Send> Handler<T> for MyHandler {
        fn invoke(&self, _req: Request<T>) -> Result<Response, HttpError> {
            Ok(Response {
                status_code: 200,
                content_type: Some("text/html".to_owned()),
                body: b"MyHandler".to_vec(),
                headers: HashMap::new(),
            })
        }
    }

    fn get_app() -> App<EmptyState> {
        let mut app = App::default();
        app.get("/", Box::new(MyHandler {}));
        app.get("/the name/:name", Box::new(MyHandler {}));
        app
    }

    #[test]
    fn dispatch_requests() {
        let app = get_app();

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 200);

        let request = app.create_request("GET", "/unknwon-path", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 404);
    }

    #[test]
    fn encoded() {
        let app = get_app();

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 200);

        let request = app.create_request("GET", "/the name/Tommaso Allevi", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 200);

        let request = app.create_request("GET", "/the%20name/Tommaso Allevi", "", b"".to_vec());
        let response = app.inject(request);
        assert_eq!(response.status_code, 200);
    }
}
