use std::path::Path;

use async_trait::async_trait;
use tokio::{fs, io::AsyncWriteExt};

use crate::{
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct File {
    path: String,
}

impl File {
    pub fn new(path: String) -> File {
        File { path }
    }

    async fn write_response<'b>(w: &mut dyn AsyncStream, status: status::Code<'b>, body: String) {
        let mut response = Response::new(status::from(status));
        response.set_header("Content-Type".into(), "text/plain".into());

        w.write_all(response.set_body(body).serialize().as_bytes())
            .await
            .unwrap();
    }
}

#[async_trait]
impl Handler for File {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        let path = Path::new(self.path.as_str()).join(&r.path()[1..]);
        let path = path.to_str().unwrap();

        info!("Serving file: {}", path);

        let contents = fs::read_to_string(path).await;

        if let Ok(contents) = contents {
            File::write_response(w, status::OK, contents).await;
            Ok(())
        } else {
            File::write_response(w, status::NOT_FOUND, "404 File not found\n".into()).await;
            Err(handler::Error::Failed("could not read file".into()))
        }
    }
}
