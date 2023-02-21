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
}

#[async_trait]
impl Handler for Stack {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        for handler in &self.handlers {
            match handler.handle(r, w).await {
                Ok(_) => {}
                Err(handler::Error::Done) => {
                    break;
                }
                Err(handler::Error::Failed(message)) => {
                    info!("Handler failed: {}", message);
                    break;
                }
            }
        }

        Ok(())
    }
}
