use async_trait::async_trait;

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    request::Request,
    router::RouteHandler,
};

#[derive(Clone, Debug)]
pub struct Stack {
    handlers: Vec<RouteHandler>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { handlers: vec![] }
    }

    pub fn push(&mut self, handler: impl Into<RouteHandler>) {
        self.handlers.push(handler.into())
    }

    pub fn push_handler(&mut self, handler: RouteHandler) {
        self.handlers.push(handler)
    }

    pub fn extend(&mut self, handlers: Vec<RouteHandler>) {
        self.handlers.append(
            handlers
                .into_iter()
                .map(|h| h.into())
                .collect::<Vec<_>>()
                .as_mut(),
        )
    }
}

/*
impl From<Stack> for RouteHandler {
    fn from(stack: Stack) -> RouteHandler {
        RouteHandler::new_unboxed(stack)
    }
}
*/

#[async_trait]
impl Handler for Stack {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut iter = self.handlers.iter();
        let mut last_result = Ok(handler::Ok::Done);
        loop {
            if let Some(handler) = iter.next() {
                last_result = handler.handler().read().await.handle(r, w).await;
                match last_result {
                    Ok(handler::Ok::Next) => {}
                    Ok(ok) => break Ok(ok),
                    Err(err) => break Err(err),
                }
            } else {
                break last_result;
            }
        }
    }
}
