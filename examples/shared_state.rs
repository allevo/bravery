use std::env;
use std::net::SocketAddr;

use bravery::{error_500, App, Handler, HttpError, Request, Response};
use std::collections::HashMap;
use std::sync::MutexGuard;
use std::sync::{Arc, Mutex};

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

#[derive(Serialize)]
struct JsonStruct<'a> {
    message: &'a str,
    counter: u32,
    other_counter: u32,
}

#[derive(Clone)]
struct TestHandler {
    other_counter: Arc<Mutex<u32>>,
}
impl Handler<Arc<Mutex<MyState>>> for TestHandler {
    fn invoke(&self, req: Request<Arc<Mutex<MyState>>>) -> Result<Response, HttpError> {
        let mut my_state: MutexGuard<MyState> =
            req.context.lock().map_err(error_500("Cannot unwrap"))?;
        my_state.counter += 1;

        let mut g = self
            .other_counter
            .lock()
            .map_err(error_500("Cannot unwrap"))?;
        *g += 1;

        let json = JsonStruct {
            message: "Hello, World!",
            counter: my_state.counter,
            other_counter: *g,
        };

        let val = serde_json::to_vec(&json).map_err(error_500("Unable to serialize"))?;

        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: val,
            headers: HashMap::new(),
        })
    }
}

struct MyState {
    counter: u32,
}

fn get_app() -> App<Arc<Mutex<MyState>>> {
    let state = MyState { counter: 0 };
    let state = Arc::new(Mutex::new(state));

    let mut app = App::new_with_state(state);
    app.get(
        "/",
        Box::new(TestHandler {
            other_counter: Arc::new(Mutex::new(0)),
        }),
    );
    app
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "127.0.0.1:8880".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    get_app().run(addr)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_body() {
        let app = get_app();

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);

        assert_eq!(response.status_code, 200);
        let expected = JsonStruct {
            message: "Hello, World!",
            other_counter: 1,
            counter: 1,
        };
        assert_eq!(response.body, serde_json::to_vec(&expected).unwrap());

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);

        assert_eq!(response.status_code, 200);
        let expected = JsonStruct {
            message: "Hello, World!",
            other_counter: 2,
            counter: 2,
        };
        assert_eq!(response.body, serde_json::to_vec(&expected).unwrap());
    }
}
