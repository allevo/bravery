use std::net::SocketAddr;
use std::env;

use std::sync::MutexGuard;
use bravery::{Handler, Request, Response, App, HttpError};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

extern crate serde;
extern crate serde_json;

#[macro_use] extern crate serde_derive;

#[derive(Serialize)]
struct JsonStruct<'a> {
  message: &'a str,
  counter: u32,
  other_counter: u32
}

struct TestHandler {
    other_counter: Mutex<u32>
}
impl Handler<Arc<Mutex<MyState>>> for TestHandler {
    fn invoke(&self, req: Request<Arc<Mutex<MyState>>>) -> Result<Response, HttpError> {
        let mut my_state: MutexGuard<MyState> = req.context.lock().unwrap();
        my_state.counter += 1;

        let mut g = self.other_counter.lock().unwrap();
        *g += 1;

        let json = JsonStruct {
            message: "Hello, World!",
            counter: my_state.counter,
            other_counter: *g
        };

        let val = serde_json::to_string(&json).unwrap();

        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: val,
            headers: HashMap::new()
        })
    }
}

struct MyState {
    counter: u32
}

fn main() -> Result<(), Box<std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8880".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let state = MyState { counter: 0 };
    let state = Arc::new(Mutex::new(state));

    let mut app = App::new_with_state(state);
    app.get("/", Box::new(TestHandler { other_counter: Mutex::new(0) }));

    app.run(addr)?;

    Ok(())
}
