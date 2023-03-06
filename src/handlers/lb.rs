use std::{
    error::{self},
    fmt,
    net::SocketAddr,
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
    net::{TcpSocket, TcpStream},
};

use crate::{
    handler::{AsyncReadStream, AsyncWriteStream},
    request::Request,
    response::Response,
};

#[derive(Debug, Clone)]
pub enum LbError {
    ConnectionError,
    SendError,
    RecvError,
    ResponseError,
}

impl fmt::Display for LbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LbError::ConnectionError => write!(f, "couldnot connect to backend"),
            LbError::SendError => write!(f, "could not send data to backend"),
            LbError::RecvError => write!(f, "could not receive data from backend"),
            LbError::ResponseError => write!(f, "could not parse response"),
        }
    }
}

impl error::Error for LbError {}

pub struct BackendState {
    writer: Option<Box<dyn AsyncWriteStream>>,
    reader: Option<Box<dyn AsyncReadStream>>,
}

impl BackendState {
    pub fn new() -> Self {
        Self {
            reader: None,
            writer: None,
        }
    }

    pub fn set_stream(&mut self, stream: TcpStream) {
        let (reader, writer) = stream.into_split();
        self.reader = Some(Box::new(reader));
        self.writer = Some(Box::new(writer));
    }
}

pub struct Backend {
    address: SocketAddr,
    state: BackendState,
}

impl Backend {
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into().parse().unwrap(),
            state: BackendState::new(),
        }
    }

    async fn connect(&mut self) -> Result<(), LbError> {
        let stream;

        if self.address.is_ipv4() {
            let socket = TcpSocket::new_v4().or(Err(LbError::ConnectionError))?;
            stream = socket
                .connect(self.address)
                .await
                .or(Err(LbError::ConnectionError))?;
        } else {
            let socket = TcpSocket::new_v6().or(Err(LbError::ConnectionError))?;
            stream = socket
                .connect(self.address)
                .await
                .or(Err(LbError::ConnectionError))?;
        }

        self.state.set_stream(stream);
        Ok(())
    }

    pub async fn send_request(&mut self, req: &Request) -> Result<Response, LbError> {
        self.connect().await?;

        let data = req.serialize();

        let f1 = self
            .state
            .writer
            .as_mut()
            .unwrap()
            .write_all(data.as_bytes());

        let mut response_bytes = String::new();

        let f2 = self
            .state
            .reader
            .as_mut()
            .unwrap()
            .read_to_string(&mut response_bytes);

        let (e1, e2) = join!(f1, f2);

        e1.or(Err(LbError::SendError))?;
        e2.or(Err(LbError::RecvError))?;

        Ok(Response::from(response_bytes).or(Err(LbError::ResponseError))?)
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
    pub fn new(backends: Vec<Backend>) -> Self {
        Lb {
            policy: Policy::RR,
            backends,
        }
    }

    pub async fn send_request(&mut self, req: &Request) -> Result<Response, LbError> {
        info!("sending request {:?}", req);
        match &mut self.policy {
            Policy::Test(backend) => backend.send_request(req).await,
            Policy::RR => self.backends[0].send_request(req).await,
            _ => self.backends[0].send_request(req).await,
        }
    }
}
