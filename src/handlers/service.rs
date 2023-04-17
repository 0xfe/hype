/// This file implements a service abstraction for handlers. It's a much simpler
/// way to construct handlers with just async functions.
use async_trait::async_trait;

use futures::{future::BoxFuture, Future};
use serde::Deserialize;

use crate::{
    handler::{Action, AsyncWriteStream, Error, Handler},
    request::Request,
};

/// A service handler is a handler that wraps an async function that takes
/// a request and returns an action. You can also pass an optional state to
/// the handler, which defaults to () if unspecified.
///
/// Actions implement From<Response>, and Responses implement from <Body>, and
/// bodies implement From<String>. If a response is not explicitly returned, the
/// handler will return a 200 OK with the stringified result of the function.
pub struct ServiceHandler<R, S = ()>
where
    R: Into<Action>,
{
    /// Wraps the async handler function.
    f: Box<dyn Fn(Request, S) -> BoxFuture<'static, Result<R, Error>> + Send + Sync>,

    /// Any arbitrary state
    state: S,
}

impl<R, S: Clone> ServiceHandler<R, S>
where
    R: Into<Action>,
{
    pub fn with_state(self, state: &S) -> Self {
        ServiceHandler {
            f: self.f,
            state: state.clone(),
        }
    }
}

#[async_trait]
impl<R: Into<Action> + Send + Sync, S: Send + Sync + Clone> Handler for ServiceHandler<R, S> {
    async fn handle(&self, r: &Request, _w: &mut dyn AsyncWriteStream) -> Result<Action, Error> {
        let result = (self.f)(r.clone(), self.state.clone()).await?;
        Ok(result.into())
    }
}

/// Create a new service handler from an async function. Use `with_state` to attach a state to the
/// service handler.
pub fn service<Func: Send + Sync, Fut, S: Default, R: Into<Action>>(
    func: Func,
) -> ServiceHandler<R, S>
where
    Func: Send + 'static + Fn(Request, S) -> Fut,
    Fut: Send + 'static + Future<Output = Result<R, Error>>,
{
    ServiceHandler {
        f: Box::new(move |a, b| Box::pin(func(a, b))),
        state: S::default(),
    }
}

pub struct FnHandler<R>
where
    R: Into<Action>,
{
    /// Wraps the async handler function.
    f: Box<dyn Fn(Request) -> BoxFuture<'static, Result<R, Error>> + Send + Sync>,
}

#[async_trait]
impl<R: Into<Action> + Send + Sync> Handler for FnHandler<R> {
    async fn handle(&self, r: &Request, _w: &mut dyn AsyncWriteStream) -> Result<Action, Error> {
        let result = (self.f)(r.clone()).await?;
        Ok(result.into())
    }
}

/// Create a new service handler from an async function. Use `with_state` to attach a state to the
/// service handler.
pub fn handler<Func: Send + Sync, Fut, R: Into<Action>>(func: Func) -> FnHandler<R>
where
    Func: Send + 'static + Fn(Request) -> Fut,
    Fut: Send + 'static + Future<Output = Result<R, Error>>,
{
    FnHandler {
        f: Box::new(move |a| Box::pin(func(a))),
    }
}

/// Helper function to deserialize a request body into a struct.
pub fn json<'de, T: Deserialize<'de>>(body: &'de Vec<u8>) -> Result<T, Error> {
    serde_json::from_str::<T>(
        std::str::from_utf8(body.as_slice()).map_err(|e| Error::Failed(e.to_string()))?,
    )
    .map_err(|e| Error::Failed(e.to_string()))
}
