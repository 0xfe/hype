use hype::cookie::Cookie;
use hype::response::*;
use hype::status;

#[test]
fn it_works_with_body() {
    let mut response = Response::new(status::from(status::OK));
    response.set_body("<HTML><b>Hello world!</b></HTML>");
    assert_eq!(
        response.serialize(),
        "HTTP/1.1 200 OK\r
content-length: 32\r
\r
<HTML><b>Hello world!</b></HTML>"
    );
}

#[test]
fn it_works_with_cookies() {
    let mut response = Response::new(status::from(status::OK));
    response.set_body("<HTML><b>Hello world!</b></HTML>");

    let mut cookie = Cookie::new("ID", "mo");
    cookie.push_flag(hype::cookie::Flag::Secure);
    cookie.push_flag(hype::cookie::Flag::Domain("mo.town".into()));

    response.set_cookie(cookie);
    response.set_cookie(Cookie::new("SID", "foobar"));

    /*
        assert_eq!(
            response.serialize(),
            "HTTP/1.1 200 OK\r
    Content-Length: 32\r
    Set-Cookie: ID=mo; Secure; Domain=mo.town\r
    Set-Cookie: SID=foobar\r
    \r
    <HTML><b>Hello world!</b></HTML>"
        );
        */
}
