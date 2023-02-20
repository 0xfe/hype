use std::collections::HashMap;

use url::Url;

#[derive(Debug, PartialEq, Eq, Hash)]
enum State {
    Start,
    InCommand,
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
        (State::InCommand, vec![State::Start]),
        (State::InHeaders, vec![State::InCommand]),
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
    BadCommandLine(String),
    BadHeaderLine(String),
    InvalidMethod(String),
    InvalidPath(String),
    UnexpectedEOF,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub base_url: String,
    pub url: Option<Url>,
    pub method: Method,
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

    fn commit_command(&mut self) -> Result<(), ParseError> {
        let command_line = std::str::from_utf8(&self.buf[..]).unwrap();
        let parts = command_line.split_ascii_whitespace().collect::<Vec<&str>>();

        if parts.len() != 3 {
            return Err(ParseError::BadCommandLine(command_line.into()));
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

        self.request.url = Some(url);
        self.request.version = parts[2].into();

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
            State::InCommand => {
                result = self.commit_command();
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
                        self.update_state(State::InCommand)?;
                    }
                }
                State::InCommand | State::InHeaders => {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_parse_result(
        buf: &str,
        parse_buf_result: Result<(), ParseError>,
        parse_eof_result: Result<(), ParseError>,
    ) -> Option<Request> {
        let mut parser = Parser::new("http://localhost".into());
        println!("Parsing buffer:\n{}", buf);
        let result1 = parser.parse_buf(String::from(buf).as_bytes());
        assert_eq!(result1, parse_buf_result);
        let result2 = parser.parse_eof();
        assert_eq!(result2, parse_eof_result);

        if result1 == Ok(()) && result2 == Ok(()) {
            return Some(parser.get_request());
        }

        None
    }

    fn assert_parse_ok(buf: &str) -> Option<Request> {
        assert_parse_result(buf, Ok(()), Ok(()))
    }

    #[test]
    fn it_works() {
        let request = assert_parse_ok(
            r##"POST / HTTP/1.1
Host: localhost:4000
Content-Length: 20

{"merchantID": "00"}"##,
        );

        assert!(request.is_some());
        let request = request.unwrap();
        assert_eq!(request.method, Method::POST);
    }

    #[test]
    fn newline_prefixes() {
        assert_parse_ok(
            r##"

POST / HTTP/1.1
Host: localhost:4000
Content-Length: 20

{"merchantID": "00"}"##,
        );
    }

    #[test]
    fn get_request() {
        let request = assert_parse_ok("GET / HTTP/1.1\n");
        assert!(request.is_some());
        let request = request.unwrap();
        assert_eq!(request.method, Method::GET);

        if let Some(url) = &request.url {
            assert_eq!(url.path(), "/");
        } else {
            assert!(&request.url.is_some())
        }
        assert_eq!(request.version, "HTTP/1.1");
    }

    #[test]
    fn invalid_method() {
        assert_parse_result(
            "BIT / HTTP/1.1\n",
            Err(ParseError::InvalidMethod("BIT".into())),
            Ok(()),
        );
    }
}
