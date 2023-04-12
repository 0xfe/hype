use async_trait::async_trait;
/// This file implements the main rx/tx logic for the network server.
use rustls_pemfile::{certs, rsa_private_keys};
use tokio::io::AsyncReadExt;

use tokio::{
    io::AsyncWriteExt,
    net::TcpListener,
    sync::{mpsc, Notify, RwLock},
};

use tokio_rustls::{
    rustls::{self, Certificate, PrivateKey},
    TlsAcceptor,
};

use crate::handler::{self, AsyncWriteStream, ErrorHandler};
use crate::headers::Headers;
use crate::parser::RequestParser;
use crate::router::{RouteHandler, Router};
use crate::{
    conntrack::{Conn, ConnTracker},
    handler::AsyncStream,
    request::Request,
    response::Response,
    router::Matcher,
    status,
};

use std::net::SocketAddr;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

/// This is the main server struct. It holds all the configuration state for the socket listener.
#[derive(Debug)]
pub struct Server {
    address: String,
    port: u16,
    base_url: String,

    /// These handlers are called based on the request path. Every handler here
    /// has a corresponding matcher to determine if it should be called.
    router: Router,

    /// This handler is called if any handler returns an error.
    error_handler: Arc<RwLock<Box<dyn ErrorHandler>>>,

    /// This tracks all the connections, and performs general housekeeping like
    /// keepalive timeouts, and closing idle connections.
    conn_tracker: Arc<RwLock<ConnTracker>>,

    /// This notifies the user that the server has started.
    start_notifier: Arc<Notify>,

    /// This notifies the user that the server has stopped.
    done_notifier: Arc<Notify>,

    /// These are used to signal the server to shutdown.
    shutdown_tx: Arc<mpsc::Sender<bool>>,
    shutdown_rx: mpsc::Receiver<bool>,

    /// TLS configuration
    enable_tls: bool,
    cert_file: PathBuf,
    key_file: PathBuf,
}

// Load TLS certs from `path`
fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut std::io::BufReader::new(std::fs::File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

// Load TLS private keys from `path`
fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    rsa_private_keys(&mut io::BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

/// This is the default error handler, which is called when a route handler returns an error.
#[derive(Debug, Clone)]
struct DefaultErrorHandler;

impl DefaultErrorHandler {
    async fn write_response<'b>(
        w: &mut dyn AsyncWriteStream,
        status: status::Code<'b>,
        content_type: String,
        body: String,
    ) -> io::Result<()> {
        let mut response = Response::new(status::from(status));
        response.headers.set("Content-Type", content_type);
        response.set_body(body.into());
        w.write_all(response.serialize().as_bytes()).await
    }
}

#[async_trait]
impl ErrorHandler for DefaultErrorHandler {
    async fn handle(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
        err: Result<handler::Ok, handler::Error>,
    ) -> Result<handler::Ok, handler::Error> {
        return match err {
            Ok(handler::Ok::Done) => Ok(handler::Ok::Done),
            Ok(handler::Ok::Next) => Ok(handler::Ok::Next),
            Ok(handler::Ok::Redirect(to)) => {
                let mut response = Response::new(status::from(status::MOVED_PERMANENTLY));
                response.headers.set("Location", to);
                w.write_all(response.serialize().as_bytes()).await.or(Err(
                    handler::Error::Failed("could not write to stream".into()),
                ))?;
                Ok(handler::Ok::Done)
            }
            Err(handler::Error::Failed(msg)) => {
                Self::write_response(
                    w,
                    status::SERVER_ERROR,
                    "text/plain".into(),
                    format!("500 INTERNAL SERVER ERROR - {}", msg),
                )
                .await
                .or(Err(handler::Error::Failed(
                    "could not write to stream".into(),
                )))?;

                Ok(handler::Ok::Done)
            }
            Err(handler::Error::Status(status)) => {
                Self::write_response(
                    w,
                    (status.code, status.text.as_ref()),
                    "text/plain".into(),
                    format!("{} {}", status.code, status.text),
                )
                .await
                .or(Err(handler::Error::Failed(
                    "could not write to stream".into(),
                )))?;

                Ok(handler::Ok::Done)
            }
            Err(handler::Error::CustomStatus(code, msg)) => {
                Self::write_response(
                    w,
                    (code, msg.as_ref()),
                    "text/plain".into(),
                    format!("{} {}", code, msg),
                )
                .await
                .or(Err(handler::Error::Failed(
                    "could not write to stream".into(),
                )))?;

                Ok(handler::Ok::Done)
            }
        };
    }
}

impl Server {
    /// Create a new server instance listening on the given address and port.
    pub fn new<T: Into<String> + Clone>(address: T, port: u16) -> Self {
        let base_url = format!("http://{}:{}", address.clone().into(), port);
        let (tx, rx) = mpsc::channel(1);

        Self {
            address: address.into(),
            port,
            router: Router::new(),
            error_handler: Arc::new(RwLock::new(Box::new(DefaultErrorHandler {}))),
            base_url,
            conn_tracker: Arc::new(RwLock::new(ConnTracker::new())),
            start_notifier: Arc::new(Notify::new()),
            done_notifier: Arc::new(Notify::new()),
            shutdown_tx: Arc::new(tx),
            shutdown_rx: rx,
            enable_tls: false,
            cert_file: PathBuf::from("localhost.crt"),
            key_file: PathBuf::from("localhost.key"),
        }
    }

    /// Enable TLS on the server using the given certificate and key files.
    pub fn enable_tls(&mut self, cert_file: PathBuf, key_file: PathBuf) {
        self.enable_tls = true;
        self.cert_file = cert_file;
        self.key_file = key_file;
    }

    /// Set the base URL for the server. This is used to generate the path and location information.
    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }

    /// Add a new handler to the server. This handler will be called if the request path matches the given path.
    pub async fn route(&self, path: impl Into<String>, handler: RouteHandler) {
        self.router.add_route(Matcher::new(&path.into()), handler);
    }

    /// Set the default handler for the server. This is called if no other handlers match the request.
    pub fn route_default(&mut self, handler: RouteHandler) {
        self.router.default_handler = handler;
    }

    /// Set the error handler for the server. This is called if any handler returns an error.
    pub async fn route_error(&mut self, handler: Box<dyn ErrorHandler>) {
        self.error_handler = Arc::new(RwLock::new(handler));
    }

    /// Get a reference to the start notifier. This is used to notify the user that the server has started.
    pub fn start_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.start_notifier)
    }

    /// Get a reference to a shutdown channel, and a done notifier. The shutdown channel is used to
    /// signal the server to shutdown. The done notifier is used to notify the user that the server
    /// has stopped.
    pub fn shutdown(&self) -> (Arc<mpsc::Sender<bool>>, Arc<Notify>) {
        (
            Arc::clone(&self.shutdown_tx),
            Arc::clone(&self.done_notifier),
        )
    }

    /// Start the server. This will block until the server is shutdown.
    pub async fn start(&mut self) -> Result<(), String> {
        let mut acceptor = None;

        if self.enable_tls {
            info!("Loading TLS certificates...");
            let certs = load_certs(&self.cert_file).map_err(|e| e.to_string())?;
            let mut keys = load_keys(&self.key_file).map_err(|e| e.to_string())?;

            let config = rustls::ServerConfig::builder()
                .with_safe_defaults()
                .with_no_client_auth()
                .with_single_cert(certs, keys.remove(0))
                .map_err(|e| e.to_string())?;
            acceptor = Some(TlsAcceptor::from(Arc::new(config)));
        }

        // Start the listener
        let hostport = format!("{}:{}", self.address, self.port);
        let listener = TcpListener::bind(&hostport)
            .await
            .map_err(|e| e.to_string())?;
        let shutdown_notifier = Arc::new(Notify::new());
        info!("Listening on {}", hostport);

        // Let callers know we're ready
        self.start_notifier.notify_one();

        // Start keepalive proccessor background thread
        self.conn_tracker.read().await.process_keepalives().await;

        // Process incoming connections in each loop iteration.
        'top: loop {
            let shutdown_notifier = Arc::clone(&shutdown_notifier);
            let conn_tracker = Arc::clone(&self.conn_tracker);
            let (tcp_socket, _) = tokio::select! {
                // Received a connection...
                result = listener.accept() => {
                    if let Err(err) = result {
                        // Don't propagate accept errors, just continue.
                        debug!("accept error: {}", err.to_string());
                        continue 'top;
                    }
                    result.unwrap()
                },

                // Received a shutdown signal...
                _ = self.shutdown_rx.recv() => {
                    shutdown_notifier.notify_one();
                    conn_tracker.read().await.shutdown();
                    info!("Shutting down...");
                    break 'top;
                }
            };

            // Got connection, setup a new ConnectedServer from the stream.
            let peer_addr = tcp_socket
                .peer_addr()
                .map_err(|e| format!("peer_addr(): {}", e))?;

            let socket: Box<dyn AsyncStream>;

            // If TLS, wrap the socket in a TLS stream.
            if let Some(ref acceptor) = acceptor {
                let acceptor = acceptor.clone();
                let connection = acceptor.accept(tcp_socket).await;
                if let Err(err) = connection {
                    // Don't propagate TLS connection errors, just continue.
                    debug!("TLS accept error: {}", err.to_string());
                    continue 'top;
                }
                socket = Box::new(connection.unwrap());
            } else {
                // No TLS, just use the raw socket.
                socket = Box::new(tcp_socket);
            }

            let conn = self.conn_tracker.write().await.push_stream(socket);
            let base_url = self.base_url.clone();
            let router = self.router.clone();
            let error_handler = Arc::clone(&self.error_handler);

            // Spawn a new task to handle the connection.
            tokio::spawn(async move {
                let mut stream = ConnectedServer {
                    conn,
                    peer_addr,
                    base_url,
                    router,
                    error_handler,
                    shutdown_notifier,
                    conn_tracker,
                    close_connection: false,
                };

                if let Err(err) = stream.process_connection().await {
                    warn!("server error: {err}");
                    _ = stream.conn.writer().write().await.shutdown().await;
                }
            });
        }

        // Let tests know we're done
        self.done_notifier.notify_one();

        Ok(())
    }
}

/// This struct represents an open HTTP stream. It's created by the server when a new
/// connection is received.
#[derive(Debug)]
struct ConnectedServer {
    /// The connection stream.
    conn: Conn,

    /// This tells the server to close the connection after the current
    /// request is processed.
    close_connection: bool,

    /// IP address of the connected peer.
    peer_addr: SocketAddr,

    /// Configuration from the server.
    base_url: String,
    router: Router,
    error_handler: Arc<RwLock<Box<dyn ErrorHandler>>>,
    shutdown_notifier: Arc<Notify>,
    conn_tracker: Arc<RwLock<ConnTracker>>,
}

impl ConnectedServer {
    /// This method processes HTTP headers for connection management.
    async fn process_headers(&mut self, headers: &Headers) {
        if let Some(connection) = headers.get_first("connection") {
            match connection.to_lowercase().as_ref() {
                // Set the keep-alive parameters of the connection. The ConnTracker will
                // shutdown the connection when the keep-alive timeout expires.
                "keep-alive" => {
                    if let Some(keepalive) = headers.get_first("keep-alive") {
                        let parts: Vec<&str> = keepalive.split(",").map(|s| s.trim()).collect();
                        for part in parts {
                            let kv: Vec<&str> = part.split('=').map(|kv| kv.trim()).collect();
                            if kv.len() == 0 {
                                break;
                            }
                            match kv[0] {
                                "timeout" => {
                                    let dur =
                                        Duration::from_secs(kv[1].parse::<u64>().unwrap_or(60));
                                    self.conn.set_keepalive_timeout(dur);
                                    self.conn_tracker
                                        .read()
                                        .await
                                        .set_keepalive_timeout(self.conn.id().clone(), dur)
                                        .await;
                                }
                                "max" => self
                                    .conn
                                    .set_keepalive_max(kv[1].parse::<usize>().unwrap_or(100)),
                                _ => {}
                            }
                        }
                    }
                }

                // Close the connection right after this reqeust.
                "close" => {
                    self.close_connection = true;
                }
                _ => {}
            }
        }
    }

    /// This method processes multiple reuqests in the same connection.
    async fn process_connection(&mut self) -> Result<(), String> {
        info!(
            "Connection ID {} received from {:?}",
            &self.conn.id(),
            self.peer_addr
        );

        // This loop is iterated over for each Request in the same connection.
        'top: loop {
            let conn = self.conn.clone();
            let reader = conn.reader();
            let writer = conn.writer();
            let timeout_notifier = self.conn.timeout_notifier();
            let shutdown_notifier = Arc::clone(&self.shutdown_notifier);

            if self.close_connection {
                // We received `Connection: close`
                _ = conn.writer().write().await.shutdown().await;
                break;
            }

            let mut parser = RequestParser::new();
            parser.set_base_url(&self.base_url);
            let mut ready = false;

            let (tx, mut rx) = mpsc::channel(1);

            // We're trying to keep the connection open here, and keep parsing requests until
            // the socket is closed.
            tokio::spawn(async move {
                // Lock the read stream for the duration of the request.
                let mut s = reader.write().await;

                // Continue to read from the socket until we can parse a complete request, including
                // the entire body.
                while !parser.is_complete() {
                    let mut buf = [0u8; 16384];

                    let result = tokio::select! {
                        r = s.read(&mut buf) => r,
                        _ = shutdown_notifier.notified() => {
                            debug!("Shutting down connection {}...", &conn.id());
                            tx.send(Err("Shutting down".to_string())).await.unwrap();
                            break;
                        }
                        _ = timeout_notifier.notified() => {
                            debug!("Keepalive timeout for connection {}...", &conn.id());
                            tx.send(Err("Keepalive timeout".to_string())).await.unwrap();
                            break;
                        }
                    };

                    match result {
                        Ok(0) => {
                            // Connection closed, exit
                            debug!("read {} bytes", 0);
                            tx.send(Err("Connection closed".to_string())).await.unwrap();
                            break;
                        }
                        Ok(n) => {
                            debug!("read {} bytes", n);
                            let result = parser.parse_buf(&buf[..n]);
                            if let Err(e) = result {
                                // Parser error, exit
                                warn!("parser error: {:?}", e);
                                tx.send(Err(e.to_string())).await.unwrap();
                                break;
                            }

                            // Received all headers, send them to the handler. The body can be
                            // streamed asynchronously.
                            if parser.ready() && !ready {
                                tx.send(Ok(parser.get_message())).await.unwrap();
                                ready = true; // send this only once
                            }

                            // We don't break here because we need to continue reading the rest
                            // of the body from the socket. The parser will continue to populate
                            // the body buffer, and set is_complete() to true when it's done.
                        }
                        Err(e) => {
                            // Socet error, exit
                            debug!("connection closed: {:?}", e);
                            tx.send(Err("Connection closed".to_string())).await.unwrap();
                            break;
                        }
                    }
                }
            });

            let message = rx.recv().await.unwrap();
            if message.is_err() {
                // Connection closed
                break;
            }

            if self.conn.inc_request_count() {
                _ = self.conn.writer().write().await.shutdown().await;
                break 'top;
            }

            // Extract the request from the parser
            let mut request: Request = message.unwrap().into();
            request
                .headers
                .set("X-Hype-Connection-ID", self.conn.id().clone());
            request.set_conn(self.conn.clone());
            self.process_headers(&request.headers).await;

            debug!("Request: {:?}", request);

            let mut s = writer.write().await;
            let result = self.router.handle(&mut request, &mut *s).await;
            self.error_handler
                .read()
                .await
                .handle(&request, &mut *s, result)
                .await
                .map_err(|e| format!("Error running error handler: {:?}", e))?;
        }

        info!("Closed connection {}", &self.conn.id());
        Ok(())
        // If we're here, then the connection is closed, there's nothing to do.
    }
}
