use async_trait::async_trait;

use crate::{
    handler::{self, AsyncStream, Handler},
    request::Request,
};

pub struct Stack {
    handlers: Vec<Box<dyn Handler>>,
}

impl Stack {
    pub fn new() -> Stack {
        Stack { handlers: vec![] }
    }

    pub fn push_handler(&mut self, handler: Box<dyn Handler>) {
        self.handlers.push(handler)
    }

    pub fn push_handlers(&mut self, handlers: &mut Vec<Box<dyn Handler>>) {
        self.handlers.append(handlers)
    }
}

#[async_trait]
impl Handler for Stack {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        for handler in self.handlers.iter() {
            match handler.handle(r, w).await {
                Ok(handler::Ok::Done) => {
                    break;
                }
                Ok(handler::Ok::Next) => {}
                Ok(handler::Ok::Redirect(_)) => {
                    todo!();
                }
                Err(handler::Error::Failed(message)) => {
                    info!("Handler failed: {}", message);
                    break;
                }
            }
        }

        Ok(handler::Ok::Done)
    }
}
