use std::collections::HashMap;

use crate::{cookie::Cookie, parser::Message, status};

#[derive(Debug, Clone)]
pub struct Response {
    pub version: String,
    pub status: status::Status,
    pub headers: HashMap<String, String>,
    pub cookies: Vec<Cookie>,
    pub body: String,
}

impl From<Message> for Response {
    fn from(value: Message) -> Self {
        if let Message::Response(r) = value {
            return r;
        }

        panic!("value is not a response")
    }
}

impl Response {
    // This is just for testing. It parses the body as a set of newline strings,
    // so will not accept raw bodies, e.g., for PUT.
    pub fn from(buf: String) -> Result<Response, String> {
        if buf.len() == 0 {
            return Err("No data in buffer".into());
        }

        let lines: Vec<String> = buf.split('\n').map(|s| s.trim().to_string()).collect();

        if lines.len() == 0 {
            return Err("Can't parse response, no data.".into());
        }

        let status_parts: Vec<String> = lines[0]
            .splitn(3, char::is_whitespace)
            .map(|s| s.to_string())
            .collect();
        if status_parts.len() < 3 {
            return Err("Bad status line".into());
        }

        let code = status_parts[1].parse::<u16>().or(Err("Bad status code"))?;

        let status = status::Status {
            code,
            text: status_parts[2].clone(),
        };

        let mut headers: HashMap<String, String> = HashMap::new();

        for line in &lines[1..] {
            if line.is_empty() {
                break;
            }

            let header: Vec<String> = line.split(':').map(|l| l.trim().to_string()).collect();
            headers.insert(header[0].clone().to_lowercase(), header[1].clone());
        }

        Ok(Response {
            version: "HTTP/1.1".into(),
            status,
            headers,
            cookies: vec![],
            body: String::new(),
        })
    }

    pub fn new(status: status::Status) -> Response {
        Response {
            version: "HTTP/1.1".to_string(),
            status,
            headers: HashMap::new(),
            cookies: vec![],
            body: String::new(),
        }
    }

    pub fn set_status(&mut self, status: status::Status) -> &mut Self {
        self.status = status;
        self
    }

    pub fn set_header(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        let key = key.into().to_lowercase();
        let value = value.into();

        if key == "set-cookie" {
            if let Ok(cookie) = Cookie::try_from(value.clone().as_str()) {
                self.set_cookie(cookie);
            }
        }

        self.headers.insert(key, value.into());
        self
    }

    pub fn set_body(&mut self, body: String) -> &mut Self {
        self.body = body;
        self
    }

    pub fn set_cookie(&mut self, cookie: Cookie) -> &mut Self {
        self.cookies.push(cookie);
        self
    }

    pub fn serialize(&mut self) -> String {
        let status_line = format!("HTTP/1.1 {} {}", self.status.code, self.status.text);
        let length = self.body.len();
        if length > 0 {
            self.set_header("Content-Length", length.to_string());
        }

        let headers: String = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<String>>()
            .join("\r\n");

        let cookie_headers: String = self
            .cookies
            .iter()
            .map(|c| c.serialize().unwrap())
            .collect::<Vec<String>>()
            .join("\r\n");

        let buf = format!(
            "{status_line}\r\n{headers}\r\n{cookie_headers}\r\n{}",
            self.body
        );

        debug!("Serializing to:\n{}", buf);

        buf
    }
}
