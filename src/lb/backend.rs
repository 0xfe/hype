use async_trait::async_trait;

use crate::{
    client::{Client, ClientError, ConnectedClient},
    request::Request,
    response::Response,
};

#[async_trait]
pub trait Backend: Send + Sync {
    fn enable_tls(&mut self, _server_name: impl Into<String>) -> &mut Self {
        self
    }
    async fn send_request(&self, req: &Request) -> Result<Response, ClientError>;
}

pub struct HttpBackend {
    address: String,
    enable_tls: bool,
    tls_server_name: String,
}

impl HttpBackend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            enable_tls: false,
            tls_server_name: String::from(""),
        }
    }

    async fn create_client(&self) -> Result<ConnectedClient, ClientError> {
        let mut client = Client::new(self.address.to_string());
        if self.enable_tls {
            client.enable_tls(&self.tls_server_name);
        }

        client.connect().await
    }
}

#[async_trait]
impl Backend for HttpBackend {
    fn enable_tls(&mut self, server_name: impl Into<String>) -> &mut Self {
        self.enable_tls = true;
        self.tls_server_name = server_name.into();
        self
    }

    async fn send_request(&self, req: &Request) -> Result<Response, ClientError> {
        if let Some(conn) = req.conn() {
            let c = conn.backend_client();
            let mut client = c.write().await;

            if let Some(client) = &mut *client {
                debug!("reusing client for {}", &self.address);
                let r = client.send_request(req).await;
                if r.is_ok() {
                    return r;
                }
            }

            debug!("creating new client for {}", &self.address);
            *client = Some(self.create_client().await?);
            client.as_mut().unwrap().send_request(req).await
        } else {
            // Request has no `conn` so no place to attach client
            debug!("creating new client for {}", &self.address);
            self.create_client().await?.send_request(req).await
        }
    }
}
