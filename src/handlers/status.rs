use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    headers::Headers,
    request::Request,
    response::Response,
    status,
};

pub struct Status {
    status: status::Status,
    body: String,
    headers: Headers,
}

impl Status {
    pub fn new(status: status::Status, message: impl Into<String>) -> Self {
        return Self {
            status,
            body: message.into(),
            headers: Headers::new(),
        };
    }
}

#[async_trait]
impl Handler for Status {
    async fn handle(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Action, handler::Error> {
        let mut response = Response::new(self.status.clone());
        response.headers = self.headers.clone();
        response.set_body(self.body.clone().into());

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Action::Done)
    }
}

#[allow(non_snake_case)]
pub fn NotFoundHandler() -> Status {
    Status::new(
        status::from(status::NOT_FOUND),
        "<html>404 Not Found</html>",
    )
}
