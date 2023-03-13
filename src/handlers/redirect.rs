use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::{
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct Redirect {
    location: String,
}

impl Redirect {
    pub fn new(location: impl Into<String>) -> Self {
        return Redirect {
            location: location.into(),
        };
    }
}

#[async_trait]
impl Handler for Redirect {
    async fn handle(
        &self,
        _r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::MOVED_PERMANENTLY));
        response.set_header("Location", self.location.clone());
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}
