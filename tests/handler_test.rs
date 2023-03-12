use async_trait::async_trait;
use hype::{
    handler::{self, AsyncStream, Error, Handler},
    request::Request,
    response::Response,
    status,
};
use tokio::io::AsyncWriteExt;

struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&mut self, _: &Request, w: &mut dyn AsyncStream) -> Result<handler::Ok, Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_header("foo", "bar");
        response.set_body("hello world!\n".into());

        w.write_all(response.serialize().as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

#[tokio::test]
async fn it_works() {
    let mut h = Box::new(MyHandler {});

    let buf = r##"POST / HTTP/1.1
Host: localhost:4000
Content-Type: application/x-www-form-urlencoded
Content-Length: 23

merchantID=2003&foo=bar"##;

    let request = Request::from(buf.to_string(), "http://localhost").unwrap();
    let mut stream: Vec<u8> = vec![];

    h.handle(&request, &mut stream).await.unwrap();

    // need to parse response because header ordering can vary
    let expected_buf = "HTTP/1.1 200 OK\r
foo: bar\r
Content-Length: 13\r
\r
hello world!
"
    .to_string();

    let response = Response::from(expected_buf).unwrap();

    assert_eq!(
        response.headers.get("foo".into()).unwrap(),
        &"bar".to_string()
    );
}
