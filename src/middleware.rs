use async_trait::async_trait;

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    request::Request,
    router::RouteHandler,
};

pub struct Stack {
    handlers: Vec<RouteHandler>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { handlers: vec![] }
    }

    pub fn push_handler(&mut self, handler: RouteHandler) {
        self.handlers.push(handler)
    }

    pub fn push_handlers(&mut self, handlers: &mut Vec<RouteHandler>) {
        self.handlers.append(handlers)
    }
}

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
