#![allow(dead_code)]

use std::fmt;

use async_trait::async_trait;
use tokio::io::AsyncWrite;

use crate::request::Request;

#[derive(Debug)]
pub enum Error {
    Failed,
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

    use super::*;
    impl AsyncStream for Vec<u8> {}

    struct MyHandler {}

    #[async_trait]
    impl Handler for MyHandler {
        async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), Error> {
            println!("boo: {:?}", r);
            let data: Vec<u8> = vec![13];
            w.write_all(&data).await.unwrap();
            Ok(())
        }
    }

    #[tokio::test]
    async fn it_works() {
        let h = Box::new(MyHandler {});

        let mut stream: Vec<u8> = vec![];
        tokio::spawn(async move {
            h.handle(&Request::new(), &mut stream).await.unwrap();
        });
    }
}
