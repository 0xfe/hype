#![allow(dead_code)]

use std::fmt;

use async_trait::async_trait;
use tokio::io::AsyncWrite;

use crate::request::Request;

#[derive(Debug)]
pub enum Error {
    Done,
    Failed(String),
}

pub trait AsyncStream: AsyncWrite + Unpin + Send + Sync {}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), Error>;
}

impl std::fmt::Debug for dyn Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandlerFn").unwrap();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;

    use crate::{request::Parser, response::Response, status};

    use super::*;
    impl AsyncStream for Vec<u8> {}

    struct MyHandler {}

    #[async_trait]
    impl Handler for MyHandler {
        async fn handle(&self, _: &Request, w: &mut dyn AsyncStream) -> Result<(), Error> {
            let mut response = Response::new(status::from(status::OK));
            response.set_header("foo".into(), "bar".into());
            response.set_body("hello world!\n".into());

            w.write_all(response.serialize().as_bytes()).await.unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn it_works() {
        let h = Box::new(MyHandler {});

        let buf = r##"POST / HTTP/1.1
Host: localhost:4000
Content-Type: application/x-www-form-urlencoded
Content-Length: 23

merchantID=2003&foo=bar"##;

        let mut parser = Parser::new("http://localhost".into());
        parser.parse_buf(String::from(buf).as_bytes()).unwrap();
        parser.parse_eof().unwrap();

        let request = parser.get_request();
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
}
