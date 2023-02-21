use std::collections::HashMap;

use crate::status;

#[derive(Debug, Clone)]
pub struct Response<'a> {
    pub status: status::Status<'a>,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl<'a> Response<'a> {
    pub fn new(status: status::Status) -> Response {
        Response {
            status,
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn set_status(&mut self, status: status::Status<'a>) -> &mut Self {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let mut response = Response::new(status::from(status::OK));
        println!("{}", response.serialize())
    }

    #[test]
    fn it_works_with_body() {
        let mut response = Response::new(status::from(status::OK));
        response.set_body("<HTML><b>Hello world!</b></HTML>".into());
        println!("{}", response.serialize())
    }
}
