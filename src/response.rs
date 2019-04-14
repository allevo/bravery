use std::collections::HashMap;

pub struct Response {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub content_type: Option<String>,
    pub body: String
}
