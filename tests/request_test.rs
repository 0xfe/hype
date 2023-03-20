use hype::request::*;

#[test]
fn it_works_with_body() {
    let mut request = Request::new("http://localhost:8080");
    request.set_method(Method::GET);
    request.set_path("/foobar");
    request.set_header("Host", "localhost:8080");
    assert_eq!(
        request.serialize(),
        "GET /foobar HTTP/1.1\r
host: localhost:8080\r
\r
"
    );
}

#[test]
fn it_works_with_cookies() {
    let mut request = Request::new("http://localhost:8080");
    request.set_method(Method::GET);
    request.set_path("/foobar");
    request.set_header("Cookie", "foo=bar; id=blah");
    assert_eq!(
        request.serialize(),
        "GET /foobar HTTP/1.1\r
cookie: foo=bar; id=blah\r
\r
"
    );
}
