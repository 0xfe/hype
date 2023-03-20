use std::sync::Arc;

use async_trait::async_trait;
use tokio::{io::AsyncWriteExt, sync::RwLock};

use crate::{
    handler::{self, AsyncStream, Handler},
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
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = self
            .lb
            .read()
            .await
            .send_request(r)
            .await
            .map_err(|e| handler::Error::Failed(e.to_string()))?;

        w.write_all(response.serialize().as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}
