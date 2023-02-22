use hype::request::*;

fn parse(
    buf: &str,
) -> (
    Option<Request>,
    Result<(), ParseError>,
    Result<(), ParseError>,
) {
    println!("Parsing buffer:\n{}", buf);
    let mut parser = Parser::new("http://localhost".into());
    let result1 = parser.parse_buf(String::from(buf).as_bytes());
    let result2 = parser.parse_eof();
    if result1 == Ok(()) && result2 == Ok(()) {
        (Some(parser.get_request()), result1, result2)
    } else {
        (None, result1, result2)
    }
}

fn assert_parse_result(
    buf: &str,
    parse_buf_result: Result<(), ParseError>,
    parse_eof_result: Result<(), ParseError>,
) -> Option<Request> {
    let (request, result1, result2) = parse(buf);
    assert_eq!(result1, parse_buf_result);
    assert_eq!(result2, parse_eof_result);
    request
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
    assert_eq!(request.method(), Method::POST);
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
    assert_eq!(request.method(), Method::GET);

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

#[test]
fn post_params() {
    let r = r##"POST / HTTP/1.1
Host: localhost:4000
Content-Type: application/x-www-form-urlencoded
Content-Length: 23

merchantID=2003&foo=bar"##;

    let request = assert_parse_ok(r);
    assert!(request.is_some());
    let post_params = request.unwrap().post_params();
    assert!(post_params.is_some());
}
