use std::collections::HashMap;

use url::Url;

use crate::{
    request::{Request, VALID_METHODS},
    response::Response,
    status,
};

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum State {
    StartRequest,
    StartResponse,
    InMethod,
    InStatusLine,
    InHeaders,
    InBody,
}

lazy_static! {
    // map of target state -> prior state(s)
    static ref STATE_MACHINE: HashMap<State, Vec<State>> = HashMap::from([
        (State::InMethod, vec![State::StartRequest]),
        (State::InStatusLine, vec![State::StartResponse]),
        (State::InHeaders, vec![State::InMethod, State::InStatusLine]),
        (State::InBody, vec![State::InHeaders]),
    ]);

}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedState,
    InvalidStateTransition,
    BadMethodLine(String),
    BadHeaderLine(String),
    BadStatusLine(String),
    InvalidMethod(String),
    InvalidPath(String),
    UnexpectedEOF,
}

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

    pub fn mut_request(&mut self) -> &mut Request {
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

    pub fn mut_response(&mut self) -> &mut Response {
        if let Message::Response(r) = self {
            return r;
        }

        panic!("message is not a response")
    }
}

#[derive(Debug)]
pub struct Parser {
    base_url: String,
    start_state: State,
    state: State,
    buf: Vec<u8>,
    message: Message,
}

impl Parser {
    pub fn new(base_url: impl Into<String>, start_state: State) -> Parser {
        let base_url = base_url.into();
        let mut message = Message::Request(Request::new(base_url.clone()));
        if start_state == State::StartResponse {
            message = Message::Response(Response::new(status::from(status::OK)));
        }

        Parser {
            base_url: base_url,
            start_state: start_state.clone(),
            state: start_state,
            buf: Vec::with_capacity(16384),
            message,
        }
    }

    fn update_state(&mut self, target_state: State) -> Result<(), ParseError> {
        if !STATE_MACHINE
            .get(&target_state)
            .unwrap()
            .contains(&self.state)
        {
            return Err(ParseError::InvalidStateTransition);
        }

        self.state = target_state;

        Ok(())
    }

    fn commit_method(&mut self) -> Result<(), ParseError> {
        let method_line = std::str::from_utf8(&self.buf[..]).unwrap();
        let parts = method_line.split_ascii_whitespace().collect::<Vec<&str>>();

        if parts.len() != 3 {
            return Err(ParseError::BadMethodLine(method_line.into()));
        }

        if let Some(method) = VALID_METHODS.get(&parts[0]) {
            self.message.mut_request().set_method(*method);
        } else {
            return Err(ParseError::InvalidMethod(parts[0].into()));
        }

        let base_url = Url::parse(&self.base_url[..])
            .or(Err(ParseError::InvalidPath(self.base_url.clone())))?;
        let url = base_url
            .join(parts[1])
            .or(Err(ParseError::InvalidPath(parts[1].into())))?;

        self.message.mut_request().version = parts[2].into();
        self.message.mut_request().url = Some(url);

        self.buf.clear();
        Ok(())
    }

    fn commit_status_line(&mut self) -> Result<(), ParseError> {
        let status_line = std::str::from_utf8(&self.buf[..]).unwrap();
        let parts = status_line
            .splitn(3, char::is_whitespace)
            .collect::<Vec<&str>>();

        if parts.len() != 3 {
            return Err(ParseError::BadStatusLine(status_line.into()));
        }

        self.message.mut_response().version = parts[0].into();

        if let Ok(code) = parts[1].to_string().parse::<u16>() {
            self.message.mut_response().set_status(status::Status {
                code,
                text: parts[2].trim().to_string(),
            });
        }

        self.buf.clear();
        Ok(())
    }

    fn commit_header(&mut self) -> Result<(), ParseError> {
        let mut result: Result<(), ParseError> = Ok(());
        let header_line = std::str::from_utf8(&self.buf[..]).unwrap();

        let headers;
        if self.start_state == State::StartResponse {
            headers = &mut self.message.mut_response().headers;
        } else {
            headers = &mut self.message.mut_request().headers;
        }

        if header_line == "\r" || header_line == "" {
            result = self.update_state(State::InBody);
        } else {
            if let Some((k, v)) = header_line.split_once(':') {
                headers.insert(k.to_string().to_lowercase(), v.trim().into());
            } else {
                result = Err(ParseError::BadHeaderLine(header_line.into()));
            }
        }

        self.buf.clear();
        result
    }

    fn commit_line(&mut self) -> Result<(), ParseError> {
        let result: Result<(), ParseError>;

        match self.state {
            State::StartRequest => result = Ok(()),
            State::StartResponse => result = Ok(()),
            State::InMethod => {
                result = self.commit_method();
                self.update_state(State::InHeaders)?;
            }
            State::InStatusLine => {
                result = self.commit_status_line();
                self.update_state(State::InHeaders)?;
            }
            State::InHeaders => {
                result = self.commit_header();
            }
            _ => {
                result = Err(ParseError::UnexpectedState);
            }
        }

        result
    }

    fn consume(&mut self, b: u8) -> Result<(), ParseError> {
        self.buf.push(b);
        Ok(())
    }

    pub fn parse_buf(&mut self, buf: &[u8]) -> Result<(), ParseError> {
        for c in buf {
            let ch = *c as char;
            match self.state {
                State::StartRequest => {
                    if !ch.is_whitespace() {
                        self.consume(*c)?;
                        self.update_state(State::InMethod)?;
                    }
                }
                State::StartResponse => {
                    if !ch.is_whitespace() {
                        self.consume(*c)?;
                        self.update_state(State::InStatusLine)?;
                    }
                }
                State::InMethod | State::InHeaders | State::InStatusLine => {
                    if ch == '\n' {
                        self.commit_line()?;
                    } else {
                        self.consume(*c)?;
                    }
                }
                State::InBody => {
                    self.consume(*c)?;
                }
            }
        }

        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        let headers;
        if self.start_state == State::StartResponse {
            headers = &self.message.response().headers;
        } else {
            headers = &self.message.request().headers;
        }

        self.state == State::InBody
            && self.buf.len()
                == headers
                    .get("content-length")
                    .unwrap_or(&"0".into())
                    .parse::<usize>()
                    .unwrap()
    }

    pub fn parse_eof(&mut self) -> Result<(), ParseError> {
        if self.state == State::InBody || self.state == State::InHeaders {
            let body = std::str::from_utf8(&self.buf[..]).unwrap().to_string();

            if self.start_state == State::StartRequest {
                self.message.mut_request().set_body(body);
            } else {
                self.message.mut_response().set_body(body);
            }
            return Ok(());
        }

        Err(ParseError::UnexpectedEOF)
    }

    pub fn get_message(&self) -> Message {
        return self.message.clone();
    }
}
