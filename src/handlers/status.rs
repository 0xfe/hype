use std::collections::HashMap;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct Status {
    status: status::Status,
    body: String,
    headers: HashMap<String, String>,
}

impl Status {
    pub fn new(status: status::Status, message: impl Into<String>) -> Self {
        return Self {
            status,
            body: message.into(),
            headers: HashMap::new(),
        };
    }

    pub fn set_header(&mut self, key: impl Into<String>, val: impl Into<String>) {
        self.headers.insert(key.into(), val.into());
    }
}

#[async_trait]
impl Handler for Status {
    async fn handle(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(self.status.clone());
        response.set_body(self.body.clone());

        for header in &self.headers {
            response.set_header(header.0, header.1);
        }
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

#[allow(non_snake_case)]
pub fn NotFoundHandler() -> Status {
    Status::new(
        status::from(status::NOT_FOUND),
        "<html>404 Not Found</html>",
    )
}
