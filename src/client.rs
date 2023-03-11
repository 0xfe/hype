use std::{error, fmt, net::SocketAddr, sync::Arc};

use tokio::{
    io::{self, AsyncReadExt, AsyncWriteExt},
    join,
    net::TcpSocket,
    sync::Mutex,
    task::JoinError,
};

use crate::{
    handler::{AsyncReadStream, AsyncWriteStream},
    parser,
    request::Request,
    response::Response,
};

#[derive(Debug)]
pub enum ClientError {
    ConnectionError,
    ConnectionBroken,
    SendError(io::Error),
    RecvError(io::Error),
    ResponseError,
    InternalError(JoinError),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::ConnectionError => write!(f, "could not connect to backend"),
            ClientError::ConnectionBroken => write!(f, "connection broken"),
            ClientError::SendError(err) => write!(f, "could not send data to backend: {}", err),
            ClientError::RecvError(err) => {
                write!(f, "could not receive data from backend: {}", err)
            }
            ClientError::ResponseError => write!(f, "could not parse response"),
            ClientError::InternalError(err) => write!(f, "could not spawn tasks: {}", err),
        }
    }
}

impl error::Error for ClientError {}

impl Client {}

#[derive(Debug)]
pub struct Client {
    address: SocketAddr,
}

impl Client {
    pub fn new(address: impl Into<String>) -> Self {
        return Self {
            address: address.into().parse().unwrap(),
        };
    }

    pub async fn connect(&mut self) -> Result<ConnectedClient, ClientError> {
        let stream;

        if self.address.is_ipv4() {
            let socket = TcpSocket::new_v4().or(Err(ClientError::ConnectionError))?;
            stream = socket
                .connect(self.address)
                .await
                .or(Err(ClientError::ConnectionError))?;
        } else {
            let socket = TcpSocket::new_v6().or(Err(ClientError::ConnectionError))?;
            stream = socket
                .connect(self.address)
                .await
                .or(Err(ClientError::ConnectionError))?;
        }

        let (reader, writer) = stream.into_split();
        Ok(ConnectedClient {
            writer: Some(Arc::new(Mutex::new(Box::new(writer)))),
            reader: Some(Arc::new(Mutex::new(Box::new(reader)))),
        })
    }
}

pub struct ConnectedClient {
    writer: Option<Arc<Mutex<Box<dyn AsyncWriteStream>>>>,
    reader: Option<Arc<Mutex<Box<dyn AsyncReadStream>>>>,
}

impl ConnectedClient {
    pub async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        let data = req.serialize();

        let writer = Arc::clone(&self.writer.as_ref().unwrap());
        let reader = Arc::clone(&self.reader.as_ref().unwrap());

        let handle1 =
            tokio::spawn(async move { writer.lock().await.write_all(data.as_bytes()).await });

        let handle2 = tokio::spawn(async move {
            let mut stream = reader.lock().await;

            let mut parser = parser::Parser::new("http://foo", parser::State::StartResponse);

            loop {
                let mut buf = [0u8; 16384];

                match stream.read(&mut buf).await {
                    Ok(0) => {
                        parser.parse_eof().unwrap();
                        break;
                    }
                    Ok(n) => {
                        parser.parse_buf(&buf[..n]).unwrap();

                        // Clients may leave the connection open, so check to see if we've
                        // got a full request in. (Otherwise, we just block.)
                        if parser.is_complete() {
                            parser.parse_eof().unwrap();
                            break;
                        }
                    }
                    Err(e) => {
                        return Err(ClientError::RecvError(e));
                    }
                }
            }

            Ok(parser.get_message())
        });

        let (e1, e2) = join!(handle1, handle2);

        e1.map_err(|e| ClientError::InternalError(e))?
            .map_err(|e| ClientError::SendError(e))?;

        let message = e2.map_err(|e| ClientError::InternalError(e))??;

        Ok(message.into())
    }
}