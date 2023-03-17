use std::{error, fmt, net::SocketAddr, sync::Arc};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    join,
    net::{lookup_host, TcpSocket},
    sync::Mutex,
};

use crate::{
    handler::{AsyncReadStream, AsyncWriteStream},
    parser::{self, Message},
    request::Request,
    response::Response,
};

#[derive(Debug, Clone)]
pub enum ClientError {
    LookupError(String),
    ConnectionError,
    ConnectionBroken,
    ConnectionClosed,
    ShutdownError(String),
    SendError(String),
    RecvError(String),
    ResponseError,
    InternalError(String),
    OtherError(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::LookupError(address) => write!(f, "could not lookup address: {}", address),
            ClientError::ConnectionError => write!(f, "could not connect to backend"),
            ClientError::ConnectionBroken => write!(f, "connection broken"),
            ClientError::ConnectionClosed => write!(f, "connection closed"),
            ClientError::ShutdownError(err) => write!(f, "error while closing socket: {}", err),
            ClientError::SendError(err) => write!(f, "could not send data to backend: {}", err),
            ClientError::RecvError(err) => {
                write!(f, "could not receive data from backend: {}", err)
            }
            ClientError::ResponseError => write!(f, "could not parse response"),
            ClientError::InternalError(err) => write!(f, "could not spawn tasks: {}", err),
            ClientError::OtherError(err) => write!(f, "could not send request: {}", err),
        }
    }
}

impl error::Error for ClientError {}

impl Client {}

#[derive(Debug)]
pub struct Client {
    address: String,
}

impl Client {
    pub fn new(address: impl Into<String>) -> Self {
        return Self {
            address: address.into(),
        };
    }

    pub async fn connect(&mut self) -> Result<ConnectedClient, ClientError> {
        let stream;

        let addresses: Vec<SocketAddr> = lookup_host(&self.address)
            .await
            .map_err(|e| ClientError::LookupError(format!("{}: {}", self.address.clone(), e)))?
            .collect();

        if addresses.len() == 0 {
            return Err(ClientError::LookupError(format!(
                "no hosts found for {}",
                self.address
            )));
        }

        let address = addresses[0];

        if address.is_ipv4() {
            let socket = TcpSocket::new_v4().or(Err(ClientError::ConnectionError))?;
            stream = socket
                .connect(address)
                .await
                .or(Err(ClientError::ConnectionError))?;
        } else {
            let socket = TcpSocket::new_v6().or(Err(ClientError::ConnectionError))?;
            stream = socket
                .connect(address)
                .await
                .or(Err(ClientError::ConnectionError))?;
        }

        let (reader, writer) = stream.into_split();
        Ok(ConnectedClient {
            writer: Arc::new(Mutex::new(Box::new(writer))),
            reader: Arc::new(Mutex::new(Box::new(reader))),
            closed: false,
        })
    }
}

pub struct ConnectedClient {
    writer: Arc<Mutex<Box<dyn AsyncWriteStream>>>,
    reader: Arc<Mutex<Box<dyn AsyncReadStream>>>,
    closed: bool,
}

impl ConnectedClient {
    pub async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        if self.closed {
            return Err(ClientError::ConnectionClosed);
        }

        let data = req.serialize();
        debug!("Sending request:\n{}", data);

        let writer = Arc::clone(&self.writer);
        let reader = Arc::clone(&self.reader);

        let handle1 = tokio::spawn(async move {
            let result = writer
                .lock()
                .await
                .write_all(data.as_bytes())
                .await
                .map_err(|e| ClientError::SendError(e.to_string()));
            (result, writer)
        });

        let handle2 = tokio::spawn(async move {
            let mut stream = reader.lock().await;

            let mut parser = parser::Parser::new("http://foo", parser::State::StartResponse);

            loop {
                let mut buf = [0u8; 16384];

                match stream.read(&mut buf).await {
                    Ok(0) => {
                        parser.parse_eof().unwrap();
                        break Ok(());
                    }
                    Ok(n) => {
                        parser.parse_buf(&buf[..n]).unwrap();
                        debug!("{}", String::from_utf8_lossy(&buf[..]));

                        // Clients may leave the connection open, so check to see if we've
                        // got a full request in. (Otherwise, we just block.)
                        if parser.is_complete() {
                            parser.parse_eof().unwrap();
                            break Ok(());
                        }
                    }
                    Err(e) => {
                        debug!("read error: {}", e);
                        break Err(ClientError::RecvError(e.to_string()));
                    }
                }
            }?;

            Ok(parser.get_message()) as Result<Message, ClientError>
        });

        let (result1, result2) = join!(handle1, handle2);

        let (e1, writer) = result1.map_err(|e| {
            self.closed = true;
            ClientError::InternalError(e.to_string())
        })?;

        let message = result2.map_err(|e| {
            self.closed = true;
            ClientError::InternalError(e.to_string())
        })?;

        if e1.is_err() || message.is_err() {
            self.closed = true;
            writer
                .lock()
                .await
                .shutdown()
                .await
                .map_err(|e| ClientError::ShutdownError(e.to_string()))?;

            if let Err(e) = e1 {
                return Err(e);
            };
            if let Err(e) = message {
                return Err(e);
            };
        }

        Ok(message.unwrap().into())
    }

    pub fn is_closed(&self) -> bool {
        return self.closed;
    }
}
