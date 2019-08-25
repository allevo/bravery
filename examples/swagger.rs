use std::net::SocketAddr;
use std::env;

use bravery::{Handler, Request, Response, App, EmptyState, HttpError, error_500};
use std::collections::HashMap;

extern crate serde;
#[macro_use] extern crate serde_json;

#[macro_use] extern crate struct2swagger;
#[macro_use] extern crate struct2swagger_derive;
use struct2swagger::{JsonSchemaDefinition, swagger_object::{SwaggerObject, ServerObject}};

#[macro_use] extern crate serde_derive;


#[macro_use]
extern crate slog;

#[derive(Serialize, Swagger)]
struct JsonStruct {
  message: String,
}

struct TestHandler {}
impl Handler<EmptyState> for TestHandler {
    fn invoke(&self, _req: Request<EmptyState>) -> Result<Response, HttpError> {
        let json = JsonStruct {
            message: "Hello, World!".to_string(),
        };

        let val = serde_json::to_string(&json).map_err(error_500("Unable to serialize"))?.into_bytes();

        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: val,
            headers: HashMap::new()
        })
    }
}
struct SwaggerHandler {
    swagger_object: SwaggerObject,
}
impl Handler<EmptyState> for SwaggerHandler {
    fn invoke(&self, _req: Request<EmptyState>) -> Result<Response, HttpError> {
        let val = serde_json::to_string(&self.swagger_object).map_err(error_500("Unable to serialize"))?.into_bytes();

        Ok(Response {
            status_code: 200,
            content_type: Some("application/json".to_string()),
            body: val,
            headers: HashMap::new()
        })
    }
}

fn get_content_type_from_extension(extension: String) -> Option<String> {
    match extension.as_ref() {
        "png" => Some("image/png".to_owned()),
        "json" => Some("application/json".to_owned()),
        "map" => Some("application/json".to_owned()),
        _ => None,
    }
}

use std::fs;
struct ServeStaticFile {
    path_fs: String,
    mount_path: String,
}
impl Handler<EmptyState> for ServeStaticFile {
    fn invoke(&self, req: Request<EmptyState>) -> Result<Response, HttpError> {

        let mut request_path = req.path.clone();
        if request_path.chars().last().unwrap() == '/' {
            request_path = request_path + "index.html";
        }

        request_path.replace_range(0..self.mount_path.len(), &self.path_fs);
        info!(req.logger, "path"; "req_path" => req.path, "fs_path" => &request_path);

        let extension_position = &request_path.rfind('.');
        let content_type = match extension_position {
            Some(pos) => {
                let mut extension = request_path.clone();
                extension.replace_range(0..=*pos, "");
                info!(req.logger, "path"; "extension" => &extension);
                get_content_type_from_extension(extension)
            },
            None => None,
        };
        let bytes = fs::read(request_path).map_err(error_500("Unable to find path"))?;
        info!(req.logger, "bytes"; "bytes" => bytes.len());

        Ok(Response {
            status_code: 200,
            content_type,
            body: bytes,
            headers: HashMap::new()
        })
    }
}

fn get_app() -> App<EmptyState> {
    let mut swagger_object = struct2swagger::swagger_object::SwaggerObject::new("Swagger example", "1.0.0");
    swagger_object.servers = Some(vec![
        ServerObject {
            url: "http://localhost:8880".to_string(),
            description: None,
            variables: None,
        }
    ]);

    let mut app: App<EmptyState> = Default::default();

    app.get("/", Box::new(TestHandler {}));
    swagger_add_router!(swagger_object, "GET", "/", 200, "the say!", JsonStruct);

    app.get("/swagger-ui*", Box::new(ServeStaticFile {
        path_fs: "./examples/swagger-ui-dist".to_string(),
        mount_path: "/swagger-ui".to_string(),
    }));
    app.get("/openapi.json", Box::new(SwaggerHandler { swagger_object }));

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
    fn swagger() {
        let app = get_app();

        let request = app.create_request("GET", "/", "", b"".to_vec());
        let response = app.inject(request);

        assert_eq!(response.status_code, 200);
        assert_eq!(response.body, serde_json::to_string(&JsonStruct { message: "Hello, World!" }).unwrap());
    }
}
