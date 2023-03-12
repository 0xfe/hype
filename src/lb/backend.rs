use std::net::SocketAddr;

use async_trait::async_trait;

use crate::{
    client::{Client, ClientError, ConnectedClient},
    request::Request,
    response::Response,
};

#[async_trait]
pub trait Backend {
    async fn connect(&mut self) -> Result<(), ClientError>;
    async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError>;
}

pub struct HttpBackend {
    address: SocketAddr,
    client: Option<ConnectedClient>,
}

impl HttpBackend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into().parse().unwrap(),
            client: None,
        }
    }
}

#[async_trait]
impl Backend for HttpBackend {
    async fn connect(&mut self) -> Result<(), crate::client::ClientError> {
        self.client = Some(Client::new(&self.address.to_string()).connect().await?);
        Ok(())
    }

    async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        self.connect().await?;
        self.client.as_mut().unwrap().send_request(req).await
    }
}
