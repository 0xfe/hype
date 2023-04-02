use std::{collections::HashMap, str::FromStr};

use url::Url;

use crate::{conntrack::Conn, message::Message, parser::RequestParser};

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
    conn: Option<Conn>,
    handler_path: Option<String>,
    base_url: String,
    url: Option<Url>,
    version: String,
    pub headers: HashMap<String, String>,
    body: String,
}

impl From<Message> for Request {
    fn from(value: Message) -> Self {
        if let Message::Request(r) = value {
            return r;
        }

        panic!("value is not a request")
    }
}

impl Request {
    pub fn new() -> Self {
        Request {
            base_url: "http://UNSET".into(),
            handler_path: None,
            url: None,
            method: Method::GET,
            version: String::new(),
            headers: HashMap::new(),
            body: String::new(),
            conn: None,
        }
    }

    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    pub fn set_handler_path(&mut self, handler: String) {
        self.handler_path = Some(handler);
    }

    pub fn handler_path(&self) -> &Option<String> {
        return &self.handler_path;
    }

    pub fn set_version(&mut self, version: impl Into<String>) -> &mut Self {
        self.version = version.into();
        self
    }

    pub fn version(&self) -> &String {
        return &self.version;
    }

    pub fn set_body(&mut self, body: String) {
        self.body = body;
    }

    pub fn set_conn(&mut self, conn: Conn) {
        self.conn = Some(conn)
    }

    pub fn conn(&self) -> Option<Conn> {
        self.conn.clone()
    }

    pub fn url(&self) -> &Option<Url> {
        return &self.url;
    }

    pub fn set_url(&mut self, url: Url) {
        self.url = Some(url)
    }

    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }

    pub fn set_path(&mut self, path: impl AsRef<str>) {
        let mut url =
            Url::from_str(&self.base_url).unwrap_or(Url::from_str("http://UNSET").unwrap());
        url.set_path(path.as_ref());
        self.url = Some(url);
    }

    pub fn set_header(&mut self, key: impl Into<String>, val: impl Into<String>) {
        self.headers.insert(key.into().to_lowercase(), val.into());
    }

    pub fn headers_mut(&mut self) -> &mut HashMap<String, String> {
        return &mut self.headers;
    }

    pub fn headers(&self) -> &HashMap<String, String> {
        return &self.headers;
    }

    pub fn from(buf: impl Into<String>) -> Result<Self, String> {
        let mut parser = RequestParser::new();
        parser
            .parse_buf(buf.into().as_bytes())
            .or(Err("could not parse buffer"))?;
        Ok(parser.get_message().into())
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
        } else {
            r.push_str("\r\n");
        }

        r
    }
}
