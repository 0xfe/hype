use std::collections::HashMap;
use std::str;

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
    InChunkedBodySize,
    InChunkedBodyContent,
    InChunkComplete,
    EndChunkedBody,
    ParseComplete,
}

lazy_static! {
    // map of target state -> prior state(s)
    static ref STATE_MACHINE: HashMap<State, Vec<State>> = HashMap::from([
        (State::InMethod, vec![State::StartRequest]),
        (State::InStatusLine, vec![State::StartResponse]),
        (State::InHeaders, vec![State::InMethod, State::InStatusLine]),
        (State::InBody, vec![State::InHeaders]),
        (State::InChunkedBodySize, vec![State::InHeaders, State::InChunkComplete]),
        (State::InChunkedBodyContent, vec![State::InChunkedBodySize]),
        (State::InChunkComplete, vec![State::InChunkedBodyContent]),
        (State::EndChunkedBody, vec![State::InChunkComplete, State::InChunkedBodySize]),
        (State::ParseComplete, vec![State::EndChunkedBody, State::InBody, State::InHeaders]),
    ]);

}

#[derive(Debug, PartialEq)]
pub enum ParseError {
    UnexpectedState,
    InvalidStateTransition(State, State),
    BadMethodLine(String),
    BadHeaderLine(String),
    BadStatusLine(String),
    InvalidMethod(String),
    InvalidPath(String),
    InvalidChunkSize,
    NonNumericChunkSize,
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

pub struct RequestParser {}

impl RequestParser {
    pub fn new() -> Parser {
        return Parser::new(State::StartRequest);
    }
}

pub struct ResponseParser {}

impl ResponseParser {
    pub fn new() -> Parser {
        return Parser::new(State::StartResponse);
    }
}

#[derive(Debug)]
pub struct Parser {
    base_url: String,
    start_state: State,
    state: State,
    buf: Vec<u8>,
    message: Message,
    expected_chunk_size: usize,
    expected_content_length: usize,
    chunk_pos: usize,
    body: Vec<u8>,
}

impl Parser {
    pub fn new(start_state: State) -> Parser {
        let mut message = Message::Request(Request::new());
        if start_state == State::StartResponse {
            message = Message::Response(Response::new(status::from(status::OK)));
        }

        Parser {
            base_url: "http://UNSET".into(),
            start_state: start_state.clone(),
            state: start_state,
            buf: Vec::with_capacity(16384),
            body: Vec::with_capacity(16384),
            message,
            expected_chunk_size: 0,
            expected_content_length: 0,
            chunk_pos: 0,
        }
    }
    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }

    fn update_state(&mut self, target_state: State) -> Result<(), ParseError> {
        if !STATE_MACHINE
            .get(&target_state)
            .unwrap()
            .contains(&self.state)
        {
            return Err(ParseError::InvalidStateTransition(
                self.state.clone(),
                target_state.clone(),
            ));
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
            let mut has_body = false;
            let mut new_state = State::InBody;

            if let Some(length) = headers.get("content-length") {
                if length.parse::<usize>().unwrap_or(0) != 0 {
                    has_body = true;
                }
            }

            if let Some(encoding) = headers.get("transfer-encoding") {
                let parts: Vec<&str> = encoding.split(',').map(|p| p.trim()).collect();
                if parts.contains(&"chunked") {
                    debug!("expecting chunked encoding");
                    new_state = State::InChunkedBodySize;
                    has_body = true;
                }
            }

            if has_body {
                result = self.update_state(new_state);
            } else {
                self.parse_eof()?;
                self.buf.clear();
                return Ok(());
            }
        } else {
            if let Some((k, v)) = header_line.split_once(':') {
                let key = k.to_lowercase();
                if key == "content-length" {
                    self.expected_content_length = v.trim().parse::<usize>().unwrap_or(0);
                }

                headers.insert(key, v.trim().into());
            } else {
                result = Err(ParseError::BadHeaderLine(header_line.into()));
            }
        }

        self.buf.clear();
        result
    }

    fn commit_chunksize(&mut self) -> Result<(), ParseError> {
        self.expected_chunk_size = usize::from_str_radix(
            str::from_utf8(&self.buf)
                .or(Err(ParseError::InvalidChunkSize))?
                .trim(),
            16,
        )
        .or(Err(ParseError::NonNumericChunkSize))?;

        self.chunk_pos = 0;
        self.buf.clear();
        Ok(())
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

    fn consume(&mut self, b: u8) {
        self.buf.push(b);
    }

    fn consume_body(&mut self, b: u8) {
        self.body.push(b);
    }

    pub fn parse_buf(&mut self, buf: &[u8]) -> Result<(), ParseError> {
        // Fast path for body
        if self.state == State::InBody {
            self.body.extend(buf);
            if self.body.len() >= self.expected_content_length {
                self.parse_eof()?;
            }
            return Ok(());
        }

        for c in buf {
            let ch = *c as char;
            match self.state {
                State::StartRequest => {
                    if !ch.is_whitespace() {
                        self.consume(*c);
                        self.update_state(State::InMethod)?;
                    }
                }
                State::StartResponse => {
                    if !ch.is_whitespace() {
                        self.consume(*c);
                        self.update_state(State::InStatusLine)?;
                    }
                }
                State::InMethod | State::InHeaders | State::InStatusLine => {
                    if ch == '\n' {
                        self.commit_line()?;
                    } else {
                        self.consume(*c);
                    }
                }
                State::InChunkedBodySize => {
                    if ch == '\n' {
                        self.commit_chunksize()?;
                        if self.expected_chunk_size == 0 {
                            self.update_state(State::EndChunkedBody)?;
                        } else {
                            self.update_state(State::InChunkedBodyContent)?;
                        }
                    } else if ch.is_ascii_hexdigit() {
                        self.consume(*c);
                    }

                    // skip anything else
                }
                State::InChunkedBodyContent => {
                    self.consume_body(*c);
                    self.chunk_pos += 1;

                    if self.chunk_pos == self.expected_chunk_size {
                        self.update_state(State::InChunkComplete)?;
                        self.buf.clear();
                    }
                }
                State::InChunkComplete => {
                    if ch == '\n' {
                        self.update_state(State::InChunkedBodySize)?;
                    }
                }
                State::InBody => {
                    self.consume_body(*c);
                    if self.body.len() == self.expected_content_length {
                        self.parse_eof()?;
                    }
                }
                State::EndChunkedBody => {
                    if ch == '\n' {
                        self.parse_eof()?;
                        break;
                    }
                }
                State::ParseComplete => {}
            }
        }

        Ok(())
    }

    pub fn is_complete(&self) -> bool {
        debug!("STATE: {:?}", self.state);
        return self.state == State::ParseComplete;
    }

    fn parse_eof(&mut self) -> Result<(), ParseError> {
        if self.state == State::ParseComplete {
            return Ok(());
        }

        if self.state == State::InBody
            || self.state == State::InHeaders
            || self.state == State::EndChunkedBody
        {
            let body = String::from_utf8_lossy(&self.body[..]);

            if self.start_state == State::StartRequest {
                self.message.mut_request().set_body(body.into_owned());
            } else {
                self.message.mut_response().set_body(body.into_owned());
            }
            self.update_state(State::ParseComplete)?;
            return Ok(());
        }

        Err(ParseError::UnexpectedEOF)
    }

    pub fn get_message(&self) -> Message {
        return self.message.clone();
    }
}
