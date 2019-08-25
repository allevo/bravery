use std::env;
use std::net::SocketAddr;

use bravery::{error_500, App, EmptyState, Handler, HttpError, Request, Response};
use std::collections::HashMap;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

#[derive(Serialize)]
struct JsonStruct<'a> {
    message: &'a str,
}

struct TestHandler {}
impl Handler<EmptyState> for TestHandler {
    fn invoke(&self, _req: Request<EmptyState>) -> Result<Response, HttpError> {
        let json = JsonStruct {
            message: "Hello, World!",
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

fn get_app() -> App<EmptyState> {
    let mut app: App<EmptyState> = Default::default();
    app.get("/", Box::new(TestHandler {}));
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
    fn hello_world() {
        let app = get_app();

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);

        assert_eq!(response.status_code, 200);
        assert_eq!(
            response.body,
            serde_json::to_vec(&JsonStruct {
                message: "Hello, World!"
            })
            .unwrap()
        );
    }
}
