use hype::parser;
use hype::parser::*;
use hype::request::*;
use hype::response::Response;

fn parse(
    buf: &str,
    start_state: parser::State,
) -> (Option<hype::parser::Message>, Result<(), ParseError>) {
    println!("Parsing buffer:\n{}", buf);
    let mut parser = Parser::new(start_state);
    let result1 = parser.parse_buf(String::from(buf).as_bytes());
    if result1 == Ok(()) {
        (Some(parser.get_message()), result1)
    } else {
        (None, result1)
    }
}

fn parse_request(buf: &str) -> (Option<Request>, Result<(), ParseError>) {
    let (message, r1) = parse(buf, parser::State::StartRequest);
    return (message.map(|m| m.into()), r1);
}

fn parse_response(buf: &str) -> (Option<Response>, Result<(), ParseError>) {
    let (message, r1) = parse(buf, parser::State::StartResponse);
    return (message.map(|m| m.into()), r1);
}

fn assert_parse_request_result(
    buf: &str,
    parse_buf_result: Result<(), ParseError>,
) -> Option<Request> {
    let (request, result1) = parse_request(buf);
    assert_eq!(result1, parse_buf_result);
    request
}

fn assert_parse_response_result(
    buf: &str,
    parse_buf_result: Result<(), ParseError>,
) -> Option<Response> {
    let (response, result1) = parse_response(buf);
    assert_eq!(result1, parse_buf_result);
    response
}

fn assert_parse_ok(buf: &str) -> Option<Request> {
    assert_parse_request_result(buf, Ok(()))
}

fn assert_parse_response_ok(buf: &str) -> Option<Response> {
    assert_parse_response_result(buf, Ok(()))
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
fn it_works_response() {
    let response = assert_parse_response_ok(
        r##"HTTP/1.1 200 OK
Host: localhost:4000
Set-Cookie: foo=bar
Content-Length: 20

{"merchantID": "00"}"##,
    );

    assert!(response.is_some());
    let response = response.unwrap();
    assert_eq!(response.status().code, 200);
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

    if let Some(url) = &request.url() {
        assert_eq!(url.path(), "/");
    } else {
        assert!(&request.url().is_some())
    }
    assert_eq!(request.version(), "HTTP/1.1");
}

#[test]
fn invalid_method() {
    assert_parse_request_result(
        "BIT / HTTP/1.1\n",
        Err(ParseError::InvalidMethod("BIT".into())),
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

    let post_params = post_params.unwrap();
    assert_eq!(post_params.get("merchantID").unwrap(), &"2003".to_string());
    assert_eq!(post_params.get("foo").unwrap(), &"bar".to_string());
}

#[test]
fn query_params() {
    let r = r##"GET /admin?user=foo&action=delete HTTP/1.1
Host: localhost:4000
Content-Type: application/x-www-form-urlencoded"##;

    let request = assert_parse_ok(r);
    assert!(request.is_some());

    let request = request.unwrap();
    assert!(request.url().is_some());
    let query_params = request.query_params();
    assert!(query_params.is_some());

    let query_params = query_params.unwrap();
    assert_eq!(query_params.get("user").unwrap(), &"foo".to_string());
    assert_eq!(query_params.get("action").unwrap(), &"delete".to_string());
}

#[test]
fn cookies() {
    let r = r##"GET /admin?user=foo&action=delete HTTP/1.1
Host: localhost:4000
Cookie: ID=mo; foo=bar; domain=boo.com
Content-Type: application/x-www-form-urlencoded"##;

    let request = assert_parse_ok(r);
    assert!(request.is_some());

    let request = request.unwrap();
    let cookies = request.cookies();
    assert!(cookies.is_some());

    let cookies = cookies.unwrap();
    assert_eq!(cookies.get("ID").unwrap(), &"mo");
    assert_eq!(cookies.get("foo").unwrap(), &"bar");
}

#[test]
fn chunked_body() {
    let response = assert_parse_response_ok(
        r##"HTTP/1.1 200 OK
Host: localhost:4000
Set-Cookie: foo=bar
Transfer-Encoding: chunked

5
12345
A
1234567890
0

"##,
    );

    assert!(response.is_some());
    let response = response.unwrap();
    assert_eq!(response.status().code, 200);
    assert_eq!(response.body(), "123451234567890");
}

#[test]
fn chunked_body1() {
    let response = assert_parse_response_ok(
        r##"HTTP/1.1 200 OK
Host: localhost:4000
Set-Cookie: foo=bar
Transfer-Encoding: chunked

0
"##,
    );

    assert!(response.is_some());
    let response = response.unwrap();
    assert_eq!(response.status().code, 200);
    assert_eq!(response.body(), "");
}
