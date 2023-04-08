use crate::{body::Body, headers::Headers, request::Request, response::Response};

#[derive(Debug, Clone)]
pub enum Message {
    None,
    Request(Request),
    Response(Response),
}

impl Message {
    pub fn request(&self) -> &Request {
        if let Message::Request(r) = self {
            return r;
        }

        panic!("message is not a request")
    }

    pub fn request_mut(&mut self) -> &mut Request {
        if let Message::Request(r) = self {
            return r;
        }

        panic!("message is not a request")
    }

    pub fn response(&self) -> &Response {
        if let Message::Response(r) = self {
            return r;
        }

        panic!("message is not a response")
    }

    pub fn response_mut(&mut self) -> &mut Response {
        if let Message::Response(r) = self {
            return r;
        }

        panic!("message is not a response")
    }

    pub fn body_mut(&mut self) -> &mut Body {
        match self {
            Message::Request(r) => &mut r.body,
            Message::Response(r) => &mut r.body,
            _ => panic!("message is not a request or response"),
        }
    }

    pub fn headers_mut(&mut self) -> &mut Headers {
        match self {
            Message::Request(r) => &mut r.headers,
            Message::Response(r) => &mut r.headers,
            _ => panic!("message is not a request or response"),
        }
    }
}
