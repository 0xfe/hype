use std::{
    error::{self},
    fmt,
    net::SocketAddr,
};

use async_trait::async_trait;
use rand::{rngs::ThreadRng, Rng};
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

#[async_trait]
pub trait Backend {
    async fn connect(&mut self) -> Result<(), crate::client::ClientError>;
    async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError>;
}

pub struct HTTPBackend {
    address: SocketAddr,
    client: Option<ConnectedClient>,
}

impl HTTPBackend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into().parse().unwrap(),
            client: None,
        }
    }
}

#[async_trait]
impl Backend for HTTPBackend {
    async fn connect(&mut self) -> Result<(), crate::client::ClientError> {
        self.client = Some(Client::new(&self.address.to_string()).connect().await?);
        Ok(())
    }

    async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        self.connect().await?;
        self.client.as_mut().unwrap().send_request(req).await
    }
}

pub trait BackendPicker<T: Backend> {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize;
}

pub struct RRPicker {
    last_index: Option<usize>,
}

impl RRPicker {
    pub fn new() -> Self {
        Self { last_index: None }
    }
}

impl<T: Backend> BackendPicker<T> for RRPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize {
        if let Some(last_index) = self.last_index {
            if last_index >= backends.len() - 1 {
                self.last_index = Some(0);
                return 0;
            } else {
                self.last_index = Some(last_index + 1);
                return last_index + 1;
            }
        }

        self.last_index = Some(0);
        0
    }
}

pub struct RandomPicker {
    rng: ThreadRng,
}

impl RandomPicker {
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }
}

impl<T: Backend> BackendPicker<T> for RandomPicker {
    fn pick_backend(&mut self, backends: &Vec<T>) -> usize {
        self.rng.gen_range(0..backends.len())
    }
}

pub enum Policy {
    Test(Box<dyn Backend>),
    RR,
    WeightedRR,
    StickyRR,
    Random,
}

impl Policy {}

pub struct Lb<T: Backend, Picker: BackendPicker<T>> {
    backends: Vec<T>,
    picker: Picker,
}

impl<T: Backend, Picker: BackendPicker<T>> Lb<T, Picker> {
    pub fn new(backends: Vec<T>, picker: Picker) -> Self {
        Lb { backends, picker }
    }

    pub async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        info!("sending request {:?}", req);
        let index = self.picker.pick_backend(&self.backends);
        self.backends[index].send_request(req).await
    }

    pub fn get_backend(&self, i: usize) -> Result<&T, String> {
        if i > self.backends.len() {
            return Err("invalid index".to_string());
        }

        Ok(&self.backends[i])
    }

    pub fn get_backend_mut(&mut self, i: usize) -> Result<&mut T, String> {
        if i > self.backends.len() {
            return Err("invalid index".to_string());
        }

        Ok(&mut self.backends[i])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        // let backend = Backend::new("142.251.33.174:80"); // google.com
        let backend = HTTPBackend::new("127.0.0.1:8080");
        let mut lb = Lb::new(vec![backend], RRPicker::new());

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
