use std::collections::HashMap;

use url::Url;

use crate::parser::{self, Parser};

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Method {
    GET,
    POST,
    PUT,
    HEAD,
    OPTIONS,
    CONNECT,
    DELETE,
    TRACE,
    PATCH,
}

lazy_static! {
    pub static ref VALID_METHODS: HashMap<&'static str, Method> = HashMap::from([
        ("GET", Method::GET),
        ("HEAD", Method::HEAD),
        ("POST", Method::POST),
        ("PUT", Method::PUT),
        ("OPTIONS", Method::OPTIONS),
        ("CONNECT", Method::CONNECT),
        ("DELETE", Method::DELETE),
        ("TRACE", Method::TRACE),
        ("PATCH", Method::PATCH),
    ]);
    pub static ref METHODS_AS_STR: HashMap<Method, &'static str> =
        VALID_METHODS.iter().map(|(k, v)| (*v, *k)).collect();
}

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    pub handler_path: Option<String>,
    pub base_url: String,
    pub url: Option<Url>,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    pub fn new(base_url: String) -> Self {
        Request {
            base_url,
            handler_path: None,
            url: None,
            method: Method::GET,
            version: String::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    pub fn set_handler_path(&mut self, handler: String) {
        self.handler_path = Some(handler);
    }

    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    pub fn from(buf: impl Into<String>, base_url: impl Into<String>) -> Result<Self, String> {
        let mut parser = Parser::new(base_url.into(), parser::State::StartRequest);
        parser
            .parse_buf(buf.into().as_bytes())
            .or(Err("could not parse buffer"))?;
        parser.parse_eof().or(Err("could not parse buffer"))?;
        Ok(parser.get_request())
    }

    pub fn post_params(&mut self) -> Option<HashMap<String, String>> {
        let mut result: HashMap<String, String> = HashMap::new();
        if let Some(content_type) = self.headers.get("content-type") {
            if *content_type == "application/x-www-form-urlencoded".to_string() {
                let parts = self.body.split('&');

                parts.for_each(|part| {
                    let kv: Vec<&str> = part.split('=').collect();
                    if kv.len() == 2 {
                        result.insert(kv[0].into(), kv[1].into());
                    }
                });
            }
            return Some(result);
        } else {
            return None;
        }
    }

    pub fn query_params(&self) -> Option<HashMap<String, String>> {
        if let Some(url) = &self.url {
            return Some(
                url.query_pairs()
                    .into_owned()
                    .collect::<HashMap<String, String>>(),
            );
        }

        None
    }

    pub fn cookies(&self) -> Option<HashMap<&str, &str>> {
        if let Some(cookies) = self.headers.get("cookie") {
            let cookies: Vec<&str> = cookies.split(';').map(|c| c.trim()).collect();

            let mut map: HashMap<&str, &str> = HashMap::new();

            cookies.iter().for_each(|c| {
                let parts = c.split('=').map(|c| c.trim()).collect::<Vec<&str>>();
                map.insert(parts[0], parts[1]);
            });

            return Some(map);
        }

        None
    }

    pub fn abs_path(&self) -> String {
        return self.url.as_ref().unwrap().path().to_string();
    }

    pub fn host(&self) -> Option<&String> {
        self.headers.get("host")
    }

    pub fn path(&self) -> String {
        if let Some(handler_path) = &self.handler_path {
            return self
                .url
                .as_ref()
                .unwrap()
                .path()
                .strip_prefix(handler_path.as_str())
                .expect("can't strip handler path")
                .to_string();
        } else {
            return self.abs_path();
        }
    }

    pub fn method(&self) -> Method {
        return self.method;
    }

    pub fn serialize(&self) -> String {
        let mut r = format!(
            "{} {} HTTP/1.1\r\n",
            METHODS_AS_STR.get(&self.method).unwrap(),
            self.abs_path()
        );

        r.push_str(
            self.headers
                .iter()
                .map(|(k, v)| format!("{}: {}", k, v))
                .collect::<Vec<String>>()
                .join("\r\n")
                .as_str(),
        );

        r.push_str("\r\n");

        if !self.body.is_empty() {
            r.push_str(format!("Content-Length: {}\r\n\r\n", self.body.chars().count()).as_str());
            r.push_str(self.body.as_str());
        }

        r
    }
}
