use std::net::SocketAddr;
use std::env;

use bravery::{Handler, Request, Response, App, EmptyState, HttpError};
use std::collections::HashMap;

extern crate serde;
extern crate serde_json;

#[macro_use] extern crate serde_derive;

#[derive(Serialize)]
struct JsonStruct<'a> {
  message: &'a str
}

fn json_error(_err: serde_json::Error) -> HttpError {
    HttpError {}
}

struct TestHandler {}
impl Handler<EmptyState> for TestHandler {
    fn invoke(&self, _req: Request<EmptyState>) -> Result<Response, HttpError> {
        let json = JsonStruct {
            message: "Hello, World!"
        };

        let val = serde_json::to_string(&json).map_err(json_error)?;

        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: val,
            headers: HashMap::new()
        })
    }
}

fn main() -> Result<(), Box<std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8880".to_string());
    let addr = addr.parse::<SocketAddr>()?;

    let mut app = App::new_with_state(EmptyState);
    app.get("/", Box::new(TestHandler {}));

    app.run(addr)?;

    Ok(())
}
