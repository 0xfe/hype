use crate::{body::Body, cookie::Cookie, headers::Headers, message::Message, status};

#[derive(Debug, Clone)]
pub struct Response {
    pub version: String,
    pub status: status::Status,
    pub headers: Headers,
    pub body: Body,
    pub cookies: Vec<Cookie>,
}

impl From<Message> for Response {
    fn from(value: Message) -> Self {
        if let Message::Response(r) = value {
            return r;
        }

        panic!("value is not a response")
    }
}

impl<T: Into<Body>> From<T> for Response {
    fn from(value: T) -> Self {
        let mut r = Response::new(status::from(status::OK));
        r.set_body(value.into());
        r
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

        let mut headers = Headers::new();

        for line in &lines[1..] {
            if line.is_empty() {
                break;
            }

            headers.add_from(line);
        }

        Ok(Response {
            version: "HTTP/1.1".into(),
            status,
            headers,
            cookies: vec![],
            body: Body::new(),
        })
    }

    pub fn new(status: status::Status) -> Response {
        Response {
            version: "HTTP/1.1".to_string(),
            status,
            headers: Headers::new(),
            cookies: vec![],
            body: Body::new(),
        }
    }

    pub fn with_body(mut self, body: impl Into<Body>) -> Self {
        self.body = body.into();
        self
    }

    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.body = body.into();
    }

    pub async fn content(&self) -> String {
        String::from_utf8_lossy(self.body.content().await.as_slice()).into()
    }

    pub fn set_cookie(&mut self, cookie: Cookie) {
        _ = cookie.serialize().and_then(|cookie| {
            self.headers.add("set-cookie", cookie);
            Ok(())
        });
    }

    pub fn set_chunked(&mut self) {
        if !self.body.chunked() {
            self.body.set_chunked();
        }

        self.headers
            .get_first_or_set("transfer-encoding", "chunked");
    }

    pub fn serialize_status(&self) -> String {
        format!("HTTP/1.1 {} {}", self.status.code, self.status.text)
    }

    pub fn serialize(&mut self) -> String {
        let status_line = format!("HTTP/1.1 {} {}", self.status.code, self.status.text);
        let length = self.body.try_content().len();
        if length > 0 {
            self.headers
                .get_first_or_set("Content-Length", length.to_string());
        }

        let headers: String = self.headers.serialize();

        let buf = format!(
            "{status_line}\r\n{headers}\r\n\r\n{}",
            String::from_utf8_lossy(self.body.try_content().as_slice())
        );

        buf
    }
}
