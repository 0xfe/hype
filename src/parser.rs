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
    InMethod,
    InHeaders,
    InBody,
    StartResponse,
    InStatusLine,
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

#[derive(Debug)]
pub struct Parser {
    base_url: String,
    start_state: State,
    state: State,
    buf: Vec<u8>,
    request: Request,
    response: Response,
}

impl Parser {
    pub fn new(base_url: String, start_state: State) -> Parser {
        Parser {
            base_url: base_url.clone(),
            start_state: start_state.clone(),
            state: start_state,
            buf: Vec::with_capacity(16384),
            request: Request::new(base_url),
            response: Response::new(status::from(status::OK)),
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
            self.request.set_method(*method);
        } else {
            return Err(ParseError::InvalidMethod(parts[0].into()));
        }

        let base_url = Url::parse(&self.base_url[..])
            .or(Err(ParseError::InvalidPath(self.base_url.clone())))?;
        let url = base_url
            .join(parts[1])
            .or(Err(ParseError::InvalidPath(parts[1].into())))?;

        self.request.version = parts[2].into();
        self.request.url = Some(url);

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

        self.response.version = parts[0].into();

        if let Ok(code) = parts[1].to_string().parse::<u16>() {
            self.response.set_status(status::Status {
                code,
                text: parts[2].to_string(),
            });
        }

        self.buf.clear();
        Ok(())
    }

    fn commit_header(&mut self) -> Result<(), ParseError> {
        let mut result: Result<(), ParseError> = Ok(());
        let header_line = std::str::from_utf8(&self.buf[..]).unwrap();

        let mut headers = &mut self.request.headers;
        if self.start_state == State::StartResponse {
            headers = &mut self.response.headers;
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
            State::InMethod => {
                result = self.commit_method();
                self.update_state(State::InHeaders)?;
            }
            State::InStatusLine => {
                result = self.commit_status_line();
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
        let mut headers = &self.request.headers;
        if self.start_state == State::StartResponse {
            headers = &self.response.headers;
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
            if self.start_state == State::StartRequest {
                self.request
                    .set_body(std::str::from_utf8(&self.buf[..]).unwrap().into());
            } else {
                self.response
                    .set_body(std::str::from_utf8(&self.buf[..]).unwrap().into());
            }
            return Ok(());
        }

        Err(ParseError::UnexpectedEOF)
    }

    pub fn get_request(&self) -> Request {
        return self.request.clone();
    }
}
