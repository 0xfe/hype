use async_trait::async_trait;

use crate::{
    client::{Client, ClientError},
    request::Request,
    response::Response,
};

#[async_trait]
pub trait Backend: Send + Sync {
    async fn send_request(&self, req: &Request) -> Result<Response, ClientError>;
}

pub struct HttpBackend {
    address: String,
}

impl HttpBackend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }
}

#[async_trait]
impl Backend for HttpBackend {
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
            *client = Some(Client::new(&self.address.to_string()).connect().await?);
            client.as_mut().unwrap().send_request(req).await
        } else {
            debug!("creating new client for {}", &self.address);
            Client::new(&self.address.to_string())
                .connect()
                .await?
                .send_request(req)
                .await
        }
    }
}
