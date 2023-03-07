use std::{
    error::{self},
    fmt,
    net::SocketAddr,
    sync::Arc,
};

use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    join,
    net::{TcpSocket, TcpStream},
    sync::Mutex,
    task::JoinError,
};

use crate::{
    handler::{AsyncReadStream, AsyncWriteStream},
    request::Request,
    response::Response,
};

#[derive(Debug)]
pub enum LbError {
    ConnectionError,
    SendError(io::Error),
    RecvError(io::Error),
    ResponseError,
    InternalError(JoinError),
}

impl fmt::Display for LbError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LbError::ConnectionError => write!(f, "couldnot connect to backend"),
            LbError::SendError(err) => write!(f, "could not send data to backend: {}", err),
            LbError::RecvError(err) => write!(f, "could not receive data from backend: {}", err),
            LbError::ResponseError => write!(f, "could not parse response"),
            LbError::InternalError(err) => write!(f, "could not spawn tasks: {}", err),
        }
    }
}

impl error::Error for LbError {}

pub struct BackendState {
    writer: Option<Arc<Mutex<Box<dyn AsyncWriteStream>>>>,
    reader: Option<Arc<Mutex<Box<dyn AsyncReadStream>>>>,
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
        self.reader = Some(Arc::new(Mutex::new(Box::new(reader))));
        self.writer = Some(Arc::new(Mutex::new(Box::new(writer))));
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

        let writer = Arc::clone(&self.state.writer.as_ref().unwrap());
        let reader = Arc::clone(&self.state.reader.as_ref().unwrap());

        let handle1 = tokio::spawn(async move {
            writer.lock().await.write_all(data.as_bytes()).await?;
            println!("HANDLER1 DONE");
            writer.lock().await.flush().await
        });

        let handle2 = tokio::spawn(async move {
            let mut response_bytes = String::new();
            reader
                .lock()
                .await
                .read_to_string(&mut response_bytes)
                .await?;

            println!("HANDLER2 DONE");
            Ok(response_bytes)
        });

        let (e1, e2) = join!(handle1, handle2);

        e1.map_err(|e| LbError::InternalError(e))?
            .map_err(|e| LbError::SendError(e))?;

        let response_bytes = e2
            .map_err(|e| LbError::InternalError(e))?
            .map_err(|e| LbError::RecvError(e))?;

        println!("{}", response_bytes);
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
    pub fn new(policy: Policy, backends: Vec<Backend>) -> Self {
        Lb { policy, backends }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn it_works() {
        // let backend = Backend::new("142.251.33.174:80"); // google.com
        let backend = Backend::new("127.0.0.1:8080"); // google.com
        let mut lb = Lb::new(Policy::RR, vec![backend]);

        let r = r##"GET / HTTP/1.1
Accept-Encoding: identity
Host: google.com"##;

        let req = Request::from(r, "http://google.com").unwrap();
        let response = lb.send_request(&req).await.unwrap();

        println!("{:?}", response);
    }
}
