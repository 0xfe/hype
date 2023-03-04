#![allow(dead_code)]

use std::{error, fmt};

use async_trait::async_trait;
use tokio::{io::AsyncWrite, net::TcpStream};

use crate::request::Request;

#[derive(Debug, Clone)]
pub enum Error {
    Done,
    Failed(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Error::Done => "Done".to_string(),
            Error::Failed(msg) => format!("Failed: {}", msg),
        };

        write!(f, "Handler error: {}", e)
    }
}

impl error::Error for Error {}

pub trait AsyncStream: AsyncWrite + Unpin + Send + Sync {}

impl AsyncStream for Vec<u8> {} // for tests
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
