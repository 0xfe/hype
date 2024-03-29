#![allow(dead_code)]

use std::{error, fmt, io::Cursor};

use async_trait::async_trait;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};

use crate::{conntrack::ConnId, request::Request, response::Response, status::Status};

/// Handlers can return a follow up action, or an error. Actions may lead too
/// another handler, or a redirect, or an immediate response. Errors always lead
/// to an immediate response.
///
/// Handlers are acted upon by server::Server and middleware::Stack.
#[derive(Debug, Clone)]
pub enum Action {
    /// Continue to next handler in the stack.
    Next,

    /// Respond immediately with the included response.
    Response(Response),

    /// Respond immediately with a 401 Redirect to a new location. Does not
    /// continue to next handler in the stack.
    Redirect(String),

    /// This session is complete. Do not continue to next handler in the stack.
    Done,
}

impl<T: Into<Response>> From<T> for Action {
    fn from(r: T) -> Self {
        Action::Response(r.into())
    }
}

/// A failed handler returns an Error.
#[derive(Debug, Clone)]
pub enum Error<S = Status>
where
    S: Into<Status> + Clone,
{
    /// Return a 500 Internal Server Error with a message.
    Failed(String),

    /// Return a status code with a standard status message.
    Status(S),
}

impl<S: Into<Status> + Clone> fmt::Display for Error<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Error::Failed(msg) => format!("Failed: {}", msg),
            Error::Status(status) => {
                let status: Status = status.clone().into();
                format!("Failed({}): {}", status.code, status.text)
            }
        };

        write!(f, "Handler error: {}", e)
    }
}

impl error::Error for Error {}

#[async_trait]
pub trait Handler: Send + Sync {
    async fn new_connection(&self, _id: ConnId) -> Result<(), Error> {
        Ok(())
    }

    async fn handle(&self, r: &Request, w: &mut dyn AsyncWriteStream) -> Result<Action, Error>;
}

#[async_trait]
pub trait ErrorHandler: Send + Sync {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
        err: Result<Action, Error>,
    ) -> Result<Action, Error>;
}

struct HandlerParams<'a, 'b>(&'a Request, &'b mut dyn AsyncWriteStream);

impl std::fmt::Debug for dyn Handler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "HandlerFn").unwrap();
        Ok(())
    }
}

impl std::fmt::Debug for dyn ErrorHandler {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ErrorHandlerFn").unwrap();
        Ok(())
    }
}

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
