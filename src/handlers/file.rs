use std::path::Path;

use async_trait::async_trait;
use tokio::io::AsyncWriteExt;

use crate::{
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct File<'a> {
    path: &'a Path,
}

impl<'a> File<'a> {
    pub fn new(path: &'a str) -> File<'a> {
        File {
            path: Path::new(path),
        }
    }

    async fn write_response<'b>(w: &mut dyn AsyncStream, status: status::Code<'b>, body: String) {
        let mut response = Response::new(status::from(status));
        w.write_all(response.set_body(body).serialize().as_bytes())
            .await
            .unwrap();
    }
}

#[async_trait]
impl<'a> Handler for File<'a> {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        File::write_response(
            w,
            status::OK,
            format!(
                "Serving file: {} from path {}\n",
                self.path
                    .join(&r.url.as_ref().unwrap().path()[1..])
                    .to_str()
                    .unwrap(),
                self.path.to_str().unwrap(),
            ),
        )
        .await;
        Ok(())
    }
}
