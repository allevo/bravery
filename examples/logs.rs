
use std::net::SocketAddr;
use std::env;

use bravery::{Handler, Request, Response, App, EmptyState, HttpError};
use std::collections::HashMap;

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate slog;

#[derive(Debug)]
#[derive(Deserialize, Serialize)]
struct MyParams {
    message: Option<String>
}

impl slog::Value for MyParams {
    fn serialize(&self, _record: &slog::Record, key: slog::Key, serializer: &mut dyn slog::Serializer) -> slog::Result {
        let msg: &str = match &self.message {
            None => "<empty>",
            Some(s) => &s[..]
        };
        serializer.emit_str(key, msg)
   }
}

struct TestHandler {}
impl Handler<EmptyState> for TestHandler {
    fn invoke(&self, req: Request<EmptyState>) -> Result<Response, HttpError> {
        let params = req.query_string_as::<MyParams>().unwrap();
        info!(req.logger, "formatted: {}", 1; "wow" => params);
        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: "Ok".to_string(),
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
    fn hello_world() {
        let app = get_app();

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, "Ok");
    }
}
