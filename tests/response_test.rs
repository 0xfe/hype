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
