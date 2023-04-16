use async_trait::async_trait;

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    request::Request,
};

#[derive(Clone, Debug)]
pub struct Log;

#[async_trait]
impl Handler for Log {
    async fn handle(
        &self,
        r: &Request,
        _w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Action, handler::Error> {
        info!("Request {}", r.url.as_ref().unwrap());
        Ok(handler::Action::Next)
    }
}
