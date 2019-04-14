use std::collections::HashMap;

pub struct Request<T: Clone> {
    pub method: String,
    pub path: String,
    pub params: String,
    pub headers: HashMap<String, String>,
    pub content_type: Option<String>,
    pub content_length: usize,
    pub header_lenght: usize,
    pub body: Vec<u8>,
    pub context: T
}

impl<C: Clone> Request<C> {
    pub fn body_as<'a, T>(&'a self) -> serde_json::Result<T>
        where T: serde::de::Deserialize<'a>
    {
        serde_json::from_slice(&self.body)
    }

    pub fn params_as<'a, T>(&'a self) -> Result<T, serde::de::value::Error>
        where T: serde::de::Deserialize<'a>
    {
        serde_urlencoded::from_str(&self.params)
    }
}
