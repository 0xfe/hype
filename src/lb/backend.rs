use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    client::{Client, ClientError, ConnectedClient},
    request::Request,
    response::Response,
};

#[async_trait]
pub trait Backend: Send + Sync {
    async fn connect(&self) -> Result<(), ClientError>;
    async fn send_request(&self, req: &Request) -> Result<Response, ClientError>;
}

pub struct HttpBackend {
    address: SocketAddr,
    client: Arc<RwLock<Option<ConnectedClient>>>,
}

impl HttpBackend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into().parse().unwrap(),
            client: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait]
impl Backend for HttpBackend {
    async fn connect(&self) -> Result<(), crate::client::ClientError> {
        *self.client.write().await = Some(Client::new(&self.address.to_string()).connect().await?);
        Ok(())
    }

    async fn send_request(&self, req: &Request) -> Result<Response, ClientError> {
        self.connect().await?;
        //self.client.as_mut().unwrap().send_request(req).await
        self.client
            .write()
            .await
            .as_mut()
            .unwrap()
            .send_request(req)
            .await
    }
}
