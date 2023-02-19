#![allow(dead_code)]

use std::fmt;

use async_trait::async_trait;
use tokio::io::AsyncWrite;

use crate::parser::Request;

#[derive(Debug)]
pub enum Error {
    Failed,
}

pub trait AsyncStream: AsyncWrite + Unpin + Send + Sync {}

#[async_trait]
pub trait HandlerFn: Send + Sync {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), Error>;
}

impl std::fmt::Debug for dyn HandlerFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandlerFn").unwrap();
        Ok(())
    }
}

#[derive(Debug)]
pub struct Handler {
    method: String,
    handler: Box<dyn HandlerFn>,
}

impl Handler {
    pub fn new(method: String, handler: Box<dyn HandlerFn>) -> Handler {
        Handler { method, handler }
    }

    pub async fn call(&self, request: &Request, stream: &mut dyn AsyncStream) -> Result<(), Error> {
        self.handler.handle(request, stream).await
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::AsyncWriteExt;

    use super::*;
    impl AsyncStream for Vec<u8> {}

    struct MyHandler {}

    #[async_trait]
    impl HandlerFn for MyHandler {
        async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), Error> {
            println!("boo: {:?}", r);
            let data: Vec<u8> = vec![13];
            w.write_all(&data).await.unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn it_works() {
        let h = Handler::new("GET".to_string(), Box::new(MyHandler {}));

        let mut stream: Vec<u8> = vec![];
        tokio::spawn(async move {
            h.call(&Request::new(), &mut stream).await.unwrap();
        });
    }
}
