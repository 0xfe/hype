use hype::cookie::Cookie;
use hype::response::*;
use hype::status;

#[test]
fn it_works() {
    let mut response = Response::new(status::from(status::OK));
    println!("{}", response.serialize())
}

#[test]
fn it_works_with_body() {
    let mut response = Response::new(status::from(status::OK));
    response.set_body("<HTML><b>Hello world!</b></HTML>".into());
    println!("{}", response.serialize())
}

#[test]
fn it_works_with_cookies() {
    let mut response = Response::new(status::from(status::OK));
    response.set_body("<HTML><b>Hello world!</b></HTML>".into());

    let mut cookie = Cookie::new("ID", "mo");
    cookie.push_flag(hype::cookie::Flag::Secure);
    cookie.push_flag(hype::cookie::Flag::Domain("mo.town".into()));

    response.push_cookie(cookie);
    response.push_cookie(Cookie::new("SID", "foobar"));
    println!("{}", response.serialize())
}
