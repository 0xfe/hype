#![allow(dead_code)]

use std::{error, fmt, io::Cursor};

use async_trait::async_trait;

use futures::{future::BoxFuture, Future};
use serde::{de::DeserializeOwned, Deserialize};
use tokio::{
    io::{AsyncRead, AsyncWrite, AsyncWriteExt},
    net::TcpStream,
};

use crate::{
    conntrack::ConnId,
    request::Request,
    response::Response,
    status::{self, Status},
};

/// Handlers can return a follow up action, or an error. Actions may lead too
/// another handler, or a redirect, or an immediate response. Errors always lead
/// to an immediate response.
///
/// Handlers are acted upon by server::Server and middleware::Stack.
#[derive(Debug, Clone)]
pub enum Action {
    /// Continue to next handler in the stack.
    Next,

    /// Respond immediately with a 401 Redirect to a new location. Does not
    /// continue to next handler in the stack.
    Redirect(String),

    /// This session is complete. Do not continue to next handler in the stack.
    Done,
}

/// A failed handler returns an Error.
#[derive(Debug, Clone)]
pub enum Error {
    /// Return a 500 Internal Server Error with a message.
    Failed(String),

    /// Return a status code with a standard status message.
    Status(Status),

    /// Return a status code with a custom message.
    CustomStatus(u16, String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Error::Failed(msg) => format!("Failed: {}", msg),
            Error::Status(status) => format!("Failed({}): {}", status.code, status.text),
            Error::CustomStatus(code, msg) => format!("Failed({}): {}", code, msg),
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

pub fn parse_json<'de, T: Deserialize<'de>>(body: &'de Vec<u8>) -> Result<T, Error> {
    serde_json::from_str::<T>(
        std::str::from_utf8(body.as_slice()).map_err(|e| Error::Failed(e.to_string()))?,
    )
    .map_err(|e| Error::Failed(e.to_string()))
}

pub struct GetHandler {
    f: Box<dyn Fn(Request) -> BoxFuture<'static, (status::Status, String)> + Send + Sync>,
}

#[async_trait]
impl Handler for GetHandler {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncWriteStream) -> Result<Action, Error> {
        let result = (self.f)(r.clone()).await;
        let mut response = Response::new(result.0);
        response.set_body(result.1.into());

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(Action::Done)
    }
}

pub fn get<Func: Send + Sync, Fut>(func: Func) -> GetHandler
where
    Func: Send + 'static + Fn(Request) -> Fut,
    Fut: Send + 'static + Future<Output = (status::Status, String)>,
{
    GetHandler {
        f: Box::new(move |a| Box::pin(func(a))),
    }
}

pub struct PostHandler<T> {
    f: Box<dyn Fn(Request, T) -> BoxFuture<'static, (status::Status, String)> + Send + Sync>,
}

#[async_trait]
impl<T: Send + Sync + DeserializeOwned> Handler for PostHandler<T> {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncWriteStream) -> Result<Action, Error> {
        let content = r.body.content().await.clone();
        let json: T = parse_json(&content)?;

        let result = (self.f)(r.clone(), json).await;
        let mut response = Response::new(result.0);
        response.set_body(result.1.into());

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(Action::Done)
    }
}

pub fn post<'de, Func: Send + Sync, Fut, T>(func: Func) -> PostHandler<T>
where
    Func: Send + 'static + Fn(Request, T) -> Fut,
    Fut: Send + 'static + Future<Output = (status::Status, String)>,
{
    PostHandler {
        f: Box::new(move |a, b| Box::pin(func(a, b))),
    }
}
