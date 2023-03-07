use std::collections::HashMap;

use url::Url;

use crate::request::{Request, VALID_METHODS};

#[derive(Debug, PartialEq, Eq, Hash)]
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
    InvalidMethod(String),
    InvalidPath(String),
    UnexpectedEOF,
}

#[derive(Debug)]
pub struct Parser {
    base_url: String,
    state: State,
    buf: Vec<u8>,
    request: Request,
}

impl Parser {
    pub fn new(base_url: String, start_state: State) -> Parser {
        Parser {
            base_url: base_url.clone(),
            state: start_state,
            buf: Vec::with_capacity(16384),
            request: Request::new(base_url),
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

    fn commit_header(&mut self) -> Result<(), ParseError> {
        let mut result: Result<(), ParseError> = Ok(());
        let header_line = std::str::from_utf8(&self.buf[..]).unwrap();

        if header_line == "\r" || header_line == "" {
            result = self.update_state(State::InBody);
        } else {
            if let Some((k, v)) = header_line.split_once(':') {
                self.request
                    .headers
                    .insert(k.to_string().to_lowercase(), v.trim().into());
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
                State::InMethod | State::InHeaders => {
                    if ch == '\n' {
                        self.commit_line()?;
                    } else {
                        self.consume(*c)?;
                    }
                }
                State::InBody => {
                    self.consume(*c)?;
                }
                _ => todo!(),
            }
        }

        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        self.state == State::InBody
            && self.buf.len()
                == self
                    .request
                    .headers
                    .get("content-length")
                    .unwrap_or(&"0".into())
                    .parse::<usize>()
                    .unwrap()
    }

    pub fn parse_eof(&mut self) -> Result<(), ParseError> {
        if self.state == State::InBody || self.state == State::InHeaders {
            self.request.body = std::str::from_utf8(&self.buf[..]).unwrap().into();
            return Ok(());
        }

        Err(ParseError::UnexpectedEOF)
    }

    pub fn get_request(&self) -> Request {
        return self.request.clone();
    }
}
