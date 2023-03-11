use std::{
    error::{self},
    fmt,
    net::SocketAddr,
};

use tokio::{
    io::{self},
    task::JoinError,
};

use crate::{
    client::{Client, ClientError, ConnectedClient},
    request::Request,
    response::Response,
};

#[derive(Debug)]
pub enum LbError {
    ConnectionError,
    ConnectionBroken,
    SendError(io::Error),
    RecvError(io::Error),
    ResponseError,
    InternalError(JoinError),
}

impl fmt::Display for LbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LbError::ConnectionError => write!(f, "could not connect to backend"),
            LbError::ConnectionBroken => write!(f, "connection broken"),
            LbError::SendError(err) => write!(f, "could not send data to backend: {}", err),
            LbError::RecvError(err) => write!(f, "could not receive data from backend: {}", err),
            LbError::ResponseError => write!(f, "could not parse response"),
            LbError::InternalError(err) => write!(f, "could not spawn tasks: {}", err),
        }
    }
}

impl error::Error for LbError {}

pub struct Backend {
    address: SocketAddr,
    client: Option<ConnectedClient>,
}

impl Backend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into().parse().unwrap(),
            client: None,
        }
    }

    async fn connect(&mut self) -> Result<(), crate::client::ClientError> {
        self.client = Some(Client::new(&self.address.to_string()).connect().await?);
        Ok(())
    }

    pub async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        self.connect().await?;
        self.client.as_mut().unwrap().send_request(req).await
    }
}

pub enum Policy {
    Test(Backend),
    RR,
    WeightedRR,
    StickyRR,
}

pub struct Lb {
    policy: Policy,
    backends: Vec<Backend>,
}

impl Lb {
    pub fn new(policy: Policy, backends: Vec<Backend>) -> Self {
        Lb { policy, backends }
    }

    pub async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        info!("sending request {:?}", req);
        match &mut self.policy {
            Policy::Test(backend) => backend.send_request(req).await,
            Policy::RR => self.backends[0].send_request(req).await,
            _ => self.backends[0].send_request(req).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        // let backend = Backend::new("142.251.33.174:80"); // google.com
        let backend = Backend::new("127.0.0.1:8080");
        let mut lb = Lb::new(Policy::RR, vec![backend]);

        let r = r##"GET / HTTP/1.1
Accept-Encoding: identity
Host: google.com"##;

        let req = Request::from(r, "http://google.com").unwrap();
        let response = lb.send_request(&req).await.unwrap();

        assert_eq!(response.status.code, 200);
        assert_eq!(response.status.text, "OK");
        assert_eq!(response.headers.get("connection").unwrap(), "keep-alive");
    }
}
