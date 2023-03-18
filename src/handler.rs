#![allow(dead_code)]

use std::{error, fmt, io::Cursor};

use async_trait::async_trait;
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::{conntrack::ConnId, request::Request};

#[derive(Debug, Clone)]
pub enum Error {
    Failed(String),
}

#[derive(Debug, Clone)]
pub enum Ok {
    Next,
    Redirect(String),
    Done,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Error::Failed(msg) => format!("Failed: {}", msg),
        };

        write!(f, "Handler error: {}", e)
    }
}

impl error::Error for Error {}

pub trait AsyncStream: AsyncWrite + Unpin + Send + Sync {}
pub trait AsyncReadStream: AsyncRead + Unpin + Send + Sync {}
pub trait AsyncWriteStream: AsyncWrite + Unpin + Send + Sync {}
pub trait AsyncRWStream: AsyncWrite + AsyncRead + Unpin + Send + Sync {}

impl AsyncStream for Vec<u8> {} // for tests
impl AsyncWriteStream for Vec<u8> {} // for tests

impl AsyncStream for TcpStream {}
impl AsyncRWStream for TcpStream {}
impl AsyncReadStream for TcpStream {}
impl AsyncWriteStream for TcpStream {}

impl AsyncReadStream for tokio::net::tcp::OwnedReadHalf {}
impl AsyncWriteStream for tokio::net::tcp::OwnedWriteHalf {}

impl AsyncRWStream for Cursor<Vec<u8>> {}
impl AsyncReadStream for Cursor<Vec<u8>> {}
impl AsyncWriteStream for Cursor<Vec<u8>> {}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn new_connection(&self, _id: ConnId) -> Result<(), Error> {
        Ok(())
    }

    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<Ok, Error>;
}

impl std::fmt::Debug for dyn Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandlerFn").unwrap();
        Ok(())
    }
}
