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

pub trait AsyncReadStream: AsyncRead + Unpin + Send + Sync {}
pub trait AsyncWriteStream: AsyncWrite + Unpin + Send + Sync {}
pub trait AsyncStream: AsyncReadStream + AsyncWriteStream {}

impl AsyncWriteStream for Vec<u8> {} // for tests

impl AsyncStream for TcpStream {}
impl AsyncReadStream for TcpStream {}
impl AsyncWriteStream for TcpStream {}

impl AsyncStream for tokio_rustls::client::TlsStream<tokio::net::TcpStream> {}
impl AsyncReadStream for tokio_rustls::client::TlsStream<tokio::net::TcpStream> {}
impl AsyncWriteStream for tokio_rustls::client::TlsStream<tokio::net::TcpStream> {}

impl AsyncStream for tokio_rustls::server::TlsStream<tokio::net::TcpStream> {}
impl AsyncReadStream for tokio_rustls::server::TlsStream<tokio::net::TcpStream> {}
impl AsyncWriteStream for tokio_rustls::server::TlsStream<tokio::net::TcpStream> {}

impl AsyncReadStream for tokio::net::tcp::OwnedReadHalf {}
impl AsyncWriteStream for tokio::net::tcp::OwnedWriteHalf {}

impl<T: AsyncReadStream> AsyncReadStream for tokio::io::ReadHalf<T> {}
impl<T: AsyncWriteStream> AsyncWriteStream for tokio::io::WriteHalf<T> {}

impl AsyncReadStream for Cursor<Vec<u8>> {}
impl AsyncWriteStream for Cursor<Vec<u8>> {}

impl AsyncStream for Box<dyn AsyncStream> {}
impl AsyncReadStream for Box<dyn AsyncStream> {}
impl AsyncWriteStream for Box<dyn AsyncStream> {}

impl AsyncReadStream for Box<dyn AsyncReadStream> {}
impl AsyncWriteStream for Box<dyn AsyncWriteStream> {}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn new_connection(&self, _id: ConnId) -> Result<(), Error> {
        Ok(())
    }

    async fn handle(&self, r: &Request, w: &mut dyn AsyncWriteStream) -> Result<Ok, Error>;
}

impl std::fmt::Debug for dyn Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandlerFn").unwrap();
        Ok(())
    }
}
