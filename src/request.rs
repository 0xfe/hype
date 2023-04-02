use std::{collections::HashMap, str::FromStr};

use url::Url;

use crate::{body::Body, conntrack::Conn, message::Message, parser::RequestParser};

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
    pub method: Method,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub url: Option<Url>,
    pub base_url: String,
    pub handler_path: Option<String>,
    pub body: Body,
    conn: Option<Conn>,
}

impl From<Message> for Request {
    fn from(value: Message) -> Self {
        if let Message::Request(r) = value {
            return r;
        }

        panic!("value is not a request")
    }
}

impl Default for Request {
    fn default() -> Self {
        Request::new(Method::GET, "/")
    }
}

impl Request {
    pub fn new(method: Method, path: impl AsRef<str>) -> Self {
        let mut request = Request {
            base_url: "http://UNSET".into(),
            handler_path: None,
            url: None,
            method,
            version: String::new(),
            headers: HashMap::new(),
            body: Body::new(),
            conn: None,
        };

        request.set_path(path);
        request
    }

    pub fn from(buf: impl Into<String>) -> Result<Self, String> {
        let mut parser = RequestParser::new();
        parser
            .parse_buf(buf.into().as_bytes())
            .or(Err("could not parse buffer"))?;
        Ok(parser.get_message().into())
    }

    pub fn set_conn(&mut self, conn: Conn) {
        self.conn = Some(conn)
    }

    pub fn conn(&self) -> Option<Conn> {
        self.conn.clone()
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

    pub fn post_params(&mut self) -> Option<HashMap<String, String>> {
        let mut result: HashMap<String, String> = HashMap::new();
        if let Some(content_type) = self.headers.get("content-type") {
            if *content_type == "application/x-www-form-urlencoded".to_string() {
                let content = self.body.full_content();
                let parts = content.split('&');

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

        let content = self.body.full_content();
        if !content.is_empty() {
            r.push_str(format!("Content-Length: {}\r\n\r\n", content.chars().count()).as_str());
            r.push_str(content.as_str());
        } else {
            r.push_str("\r\n");
        }

        r
    }
}
