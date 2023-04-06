use std::sync::Arc;

use async_trait::async_trait;
use futures::StreamExt;
use tokio::{io::AsyncWriteExt, sync::RwLock};

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    lb::{backend::HttpBackend, http::Http, picker::Picker},
    request::Request,
};

pub struct Lb<P: Picker<HttpBackend>> {
    lb: Arc<RwLock<Http<HttpBackend, P>>>,
}

impl<P: Picker<HttpBackend>> Lb<P> {
    pub fn new(balancer: Http<HttpBackend, P>) -> Self {
        return Self {
            lb: Arc::new(RwLock::new(balancer)),
        };
    }
}

#[async_trait]
impl<P: Picker<HttpBackend> + Sync + Send> Handler for Lb<P> {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let response = self
            .lb
            .read()
            .await
            .send_request(r)
            .await
            .map_err(|e| handler::Error::Failed(e.to_string()))?;

        // Write response headers
        w.write_all(response.serialize_headers().as_bytes())
            .await
            .unwrap();

        // Write body delimeter
        w.write_all("\r\n\r\n".as_bytes()).await.unwrap();

        // Write body
        if response.body.chunked() {
            let mut stream = response.body.chunk_stream();
            while let Some(chunk) = stream.next().await {
                w.write_all(chunk.as_slice()).await.unwrap();
            }
        } else {
            let mut stream = response.body.content_stream();
            while let Some(content) = stream.next().await {
                w.write_all(content.as_slice()).await.unwrap();
            }
        }
        Ok(handler::Ok::Done)
    }
}
