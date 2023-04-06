use std::{error, fmt, net::SocketAddr, sync::Arc};

use futures::StreamExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{lookup_host, TcpSocket},
    sync::{mpsc, Mutex},
};
use tokio_rustls::{rustls, TlsConnector};

use crate::{
    handler::{AsyncReadStream, AsyncWriteStream},
    parser::{self},
    request::Request,
    response::Response,
};

/// Errors returned by the client.
#[derive(Debug, Clone)]
pub enum ClientError {
    /// Errors related to DNS lookups
    LookupError(String),

    /// Errors related to the TCP connection
    ConnectionError,
    ConnectionBroken,
    ConnectionClosed,

    /// Parse errors
    ParseError(String),

    /// TLS Errors
    TLSError(String),

    /// Errors while sending or receiving data
    SendError(String),
    RecvError(String),

    /// Error closing connection
    ShutdownError(String),

    /// Other unexpected condition
    InternalError(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ClientError::LookupError(address) => write!(f, "could not lookup address: {}", address),
            ClientError::ConnectionError => write!(f, "could not connect to backend"),
            ClientError::ConnectionBroken => write!(f, "connection broken"),
            ClientError::ConnectionClosed => write!(f, "connection closed"),
            ClientError::TLSError(message) => write!(f, "TLS error: {}", message),
            ClientError::ShutdownError(err) => write!(f, "error while closing socket: {}", err),
            ClientError::SendError(err) => write!(f, "could not send data to backend: {}", err),
            ClientError::ParseError(err) => write!(f, "could not parse response: {}", err),
            ClientError::RecvError(err) => {
                write!(f, "could not receive data from backend: {}", err)
            }
            ClientError::InternalError(err) => write!(f, "internal error: {}", err),
        }
    }
}

impl error::Error for ClientError {}

impl Client {}

#[derive(Debug)]
pub struct Client {
    address: String,
    enable_tls: bool,
    tls_server_name: String,
}

impl Client {
    /// Create a new HTTP client connection.
    ///
    /// # Arguments
    /// `address` - must be a host:port. The host can be an IPv4 address,
    ///             IPv6 address, or a DNS host name.
    ///
    /// # Example
    ///
    /// ```
    /// use hype::client::Client;
    /// let client = Client::new("localhost:8080");
    /// ```
    pub fn new(address: impl Into<String>) -> Self {
        return Self {
            address: address.into(),
            enable_tls: false,
            tls_server_name: String::from(""),
        };
    }

    pub fn enable_tls(&mut self, server_name: impl Into<String>) -> &mut Self {
        self.enable_tls = true;
        self.tls_server_name = server_name.into();
        self
    }

    /// Connect to address and return a `ConnectedClient`.
    pub async fn connect(&mut self) -> Result<ConnectedClient, ClientError> {
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
        let tcp_stream;

        if address.is_ipv4() {
            let socket = TcpSocket::new_v4().or(Err(ClientError::ConnectionError))?;
            tcp_stream = socket
                .connect(address)
                .await
                .or(Err(ClientError::ConnectionError))?;
        } else {
            let socket = TcpSocket::new_v6().or(Err(ClientError::ConnectionError))?;
            tcp_stream = socket
                .connect(address)
                .await
                .or(Err(ClientError::ConnectionError))?;
        }

        if self.enable_tls {
            let mut root_cert_store = rustls::RootCertStore::empty();
            root_cert_store.add_server_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.0.iter().map(
                |ta| {
                    rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                        ta.subject,
                        ta.spki,
                        ta.name_constraints,
                    )
                },
            ));

            let config = rustls::ClientConfig::builder()
                .with_safe_defaults()
                .with_root_certificates(root_cert_store)
                .with_no_client_auth(); // i guess this was previously the default?
            let connector = TlsConnector::from(Arc::new(config));
            let domain = rustls::ServerName::try_from(self.tls_server_name.as_str())
                .map_err(|e| ClientError::TLSError(format!("invalid domain: {}", e.to_string())))?;

            let tls_stream = connector
                .connect(domain, tcp_stream)
                .await
                .map_err(|e| ClientError::TLSError(format!("connection failed {}", e)))?;

            let (reader, writer) = tokio::io::split(tls_stream);
            Ok(ConnectedClient {
                writer: Arc::new(Mutex::new(Box::new(writer))),
                reader: Arc::new(Mutex::new(Box::new(reader))),
                closed: Arc::new(Mutex::new(false)),
            })
        } else {
            let (reader, writer) = tokio::io::split(tcp_stream);
            Ok(ConnectedClient {
                writer: Arc::new(Mutex::new(Box::new(writer))),
                reader: Arc::new(Mutex::new(Box::new(reader))),
                closed: Arc::new(Mutex::new(false)),
            })
        }
    }
}

pub struct ConnectedClient {
    writer: Arc<Mutex<Box<dyn AsyncWriteStream>>>,
    reader: Arc<Mutex<Box<dyn AsyncReadStream>>>,
    closed: Arc<Mutex<bool>>,
}

impl ConnectedClient {
    pub async fn send_request(&mut self, req: &Request) -> Result<Response, ClientError> {
        if *self.closed.lock().await {
            return Err(ClientError::ConnectionClosed);
        }

        let writer = Arc::clone(&self.writer);
        let closed = Arc::clone(&self.closed);
        let reader = Arc::clone(&self.reader);
        let request_data = req.serialize_headers();

        let mut read_stream = req.body.stream();

        tokio::spawn(async move {
            let mut write_stream = writer.lock().await;
            debug!("sending request:\n{}", request_data);

            if let Err(e) = write_stream
                .write_all(format!("{}\r\n\r\n", request_data).as_bytes())
                .await
            {
                warn!("error writing to socket: {}", e);
                *closed.lock().await = true;
                _ = write_stream.shutdown().await;
            }

            while let Some(content) = read_stream.next().await {
                if let Err(e) = write_stream.write_all(content.as_slice()).await {
                    warn!("error writing chunk to socket: {}", e);
                    *closed.lock().await = true;
                    _ = write_stream.shutdown().await;
                }
            }
        });

        let (tx, mut rx) = mpsc::channel(1);

        // Background task to read the response. Returns the response struct as soon
        // as the headers are read, and continues to read from the socket in the background
        // until the entire response is read or the connection is closed.
        tokio::spawn(async move {
            let mut stream = reader.lock().await;

            let mut parser = parser::ResponseParser::new();
            let mut ready = false;

            loop {
                let mut buf = [0u8; 16384];

                match stream.read(&mut buf).await {
                    Ok(0) => {
                        debug!("0 bytes read");
                        _ = tx.send(Err(ClientError::ConnectionClosed)).await;
                        break;
                    }
                    Ok(n) => {
                        debug!("client received: {}", String::from_utf8_lossy(&buf[..n]));
                        if let Err(e) = parser.parse_buf(&buf[..n]) {
                            _ = tx.send(Err(ClientError::ParseError(e.to_string()))).await;
                            break;
                        }

                        // No need to wait for a full request. Wait until there's enough data
                        // in the buffer to parse the headers.
                        if parser.ready() && !ready {
                            _ = tx.send(Ok(parser.get_message())).await;
                            ready = true; // only send the message once
                        }

                        if parser.is_complete() {
                            break;
                        }
                    }
                    Err(e) => {
                        debug!("read error: {}", e);
                        _ = tx.send(Err(ClientError::RecvError(e.to_string()))).await;
                        break;
                    }
                }
            }
        });

        let message = rx.recv().await;

        if let Some(message) = message {
            // Error receiving data, shut down the socket
            if message.is_err() {
                self.close().await?;
            }
            Ok(message
                .map_err(|e| {
                    ClientError::InternalError(format!(
                        "error receiving response: {}",
                        e.to_string()
                    ))
                })?
                .into())
        } else {
            self.close().await?;
            Err(ClientError::InternalError(
                "error receiving response".to_string(),
            ))
        }
    }

    async fn close_internal(
        writer: Arc<Mutex<Box<dyn AsyncWriteStream>>>,
    ) -> Result<(), ClientError> {
        Ok(writer
            .lock()
            .await
            .shutdown()
            .await
            .map_err(|e| ClientError::ShutdownError(e.to_string()))?)
    }

    pub async fn close(&mut self) -> Result<(), ClientError> {
        *self.closed.lock().await = true;
        Self::close_internal(Arc::clone(&self.writer)).await
    }

    pub async fn is_closed(&self) -> bool {
        return *self.closed.lock().await;
    }
}
