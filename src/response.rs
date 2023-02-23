use std::collections::HashMap;

use crate::status;

#[derive(Debug, Clone)]
pub struct Response {
    pub status: status::Status,
    pub headers: HashMap<String, String>,
    pub body: String,
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

        let status_parts: Vec<String> =
            lines[0].split_whitespace().map(|s| s.to_string()).collect();
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
            headers.insert(header[0].clone(), header[1].clone());
        }

        Ok(Response {
            status,
            headers,
            body: String::new(),
        })
    }

    pub fn new(status: status::Status) -> Response {
        Response {
            status,
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn set_status(&mut self, status: status::Status) -> &mut Self {
        self.status = status;
        self
    }

    pub fn set_header(&mut self, key: String, value: String) -> &mut Self {
        self.headers.insert(key, value);
        self
    }

    pub fn set_body(&mut self, body: String) -> &mut Self {
        self.body = body;
        self
    }

    pub fn serialize(&mut self) -> String {
        let status_line = format!("HTTP/1.1 {} {}", self.status.code, self.status.text);
        let length = self.body.len();
        if length > 0 {
            self.set_header("Content-Length".into(), length.to_string());
        }

        let headers: String = self
            .headers
            .iter()
            .map(|(k, v)| format!("{}: {}", k, v))
            .collect::<Vec<String>>()
            .join("\r\n");

        format!("{status_line}\r\n{headers}\r\n\r\n{}", self.body)
    }
}
