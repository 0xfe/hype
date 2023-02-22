use std::collections::HashMap;

use url::Url;

#[derive(Debug, PartialEq, Eq, Hash)]
enum State {
    Start,
    InMethod,
    InHeaders,
    InBody,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
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
    // map of target state -> prior state(s)
    static ref STATE_MACHINE: HashMap<State, Vec<State>> = HashMap::from([
        (State::InMethod, vec![State::Start]),
        (State::InHeaders, vec![State::InMethod]),
        (State::InBody, vec![State::InHeaders]),
    ]);

    static ref VALID_METHODS: HashMap<&'static str, Method> = HashMap::from([
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

#[derive(Debug, Clone)]
pub struct Request {
    method: Method,
    pub base_url: String,
    pub url: Option<Url>,
    pub version: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl Request {
    pub fn new(base_url: String) -> Request {
        Request {
            base_url,
            url: None,
            method: Method::GET,
            version: String::new(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }

    pub fn post_params(&mut self) -> Option<HashMap<String, String>> {
        let mut result: HashMap<String, String> = HashMap::new();
        if let Some(content_type) = self.headers.get("Content-Type") {
            if *content_type == "application/x-www-form-urlencoded".to_string() {
                let parts = self.body.split('&');

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
            Some(
                url.query_pairs()
                    .into_owned()
                    .collect::<HashMap<String, String>>(),
            );
        }

        None
    }

    pub fn path(&self) -> &str {
        return self.url.as_ref().unwrap().path();
    }

    pub fn method(&self) -> Method {
        return self.method;
    }
}

#[derive(Debug)]
pub struct Parser {
    base_url: String,
    state: State,
    buf: Vec<u8>,
    request: Request,
}

impl Parser {
    pub fn new(base_url: String) -> Parser {
        Parser {
            base_url: base_url.clone(),
            state: State::Start,
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
            self.request.method = *method;
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
                self.request.headers.insert(k.into(), v.trim().into());
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
            State::Start => result = Ok(()),
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
                State::Start => {
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
                    .get("Content-Length")
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

    pub fn reset(&mut self) {
        self.state = State::Start;
        self.buf.clear();
        self.request = Request::new(self.base_url.clone());
    }
}
