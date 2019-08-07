use std::net::SocketAddr;
use std::env;

use bravery::{Handler, Request, Response, App, EmptyState, HttpError, error_500};
use std::collections::HashMap;

extern crate serde;
extern crate serde_json;

#[macro_use] extern crate serde_derive;

#[derive(Serialize)]
struct JsonStruct<'a> {
  message: &'a str
}
#[derive(Deserialize)]
struct MyParams {
  pub message: String
}

struct TestHandler {}
impl Handler<EmptyState> for TestHandler {
    fn invoke(&self, req: Request<EmptyState>) -> Result<Response, HttpError> {
        let params: MyParams = req.query_string_as().map_err(error_500("Unable to deserialize query_params"))?;

        let json = JsonStruct {
            message: &params.message
        };

        let val = serde_json::to_string(&json).map_err(error_500("Unable to serialize"))?;

        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: val,
            headers: HashMap::new()
        })
    }
}

fn get_app() -> App<EmptyState> {
    let mut app: App<EmptyState> = Default::default();
    app.get("/", Box::new(TestHandler {}));
    app
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = env::args().nth(1).unwrap_or_else(|| "127.0.0.1:8880".to_string());
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

        let request = app.create_request("GET", "/", "message=my_message", b"".to_vec());
        let response = app.inject(request);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, serde_json::to_string(&JsonStruct { message: "my_message" }).unwrap());
    }
}
