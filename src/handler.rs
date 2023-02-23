#![allow(dead_code)]

use std::fmt;

use async_trait::async_trait;
use tokio::{io::AsyncWrite, net::TcpStream};

use crate::request::Request;

#[derive(Debug)]
pub enum Error {
    Done,
    Failed(String),
}

pub trait AsyncStream: AsyncWrite + Unpin + Send + Sync {}

impl AsyncStream for Vec<u8> {}
impl AsyncStream for TcpStream {}

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
