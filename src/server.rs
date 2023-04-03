#![allow(non_snake_case)]

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

use crate::parser::RequestParser;
use crate::{
    conntrack::{Conn, ConnTracker},
    handler::{AsyncStream, Handler},
    request::Request,
    response::Response,
    router::Matcher,
    status,
};

use std::{
    collections::HashMap,
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

#[derive(Debug)]
pub struct Server {
    address: String,
    port: u16,
    base_url: String,
    handlers: Arc<RwLock<Vec<(Matcher, Box<dyn Handler>)>>>,
    default_handler: Option<Arc<RwLock<Box<dyn Handler>>>>,
    conn_tracker: Arc<RwLock<ConnTracker>>,
    start_notifier: Arc<Notify>,
    done_notifier: Arc<Notify>,
    shutdown_tx: Arc<mpsc::Sender<bool>>,
    shutdown_rx: mpsc::Receiver<bool>,

    // TLS configuration
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

impl Server {
    pub fn new<T: Into<String> + Clone>(address: T, port: u16) -> Self {
        let base_url = format!("http://{}:{}", address.clone().into(), port);
        let (tx, rx) = mpsc::channel(1);

        Self {
            address: address.into(),
            port,
            handlers: Arc::new(RwLock::new(Vec::new())),
            default_handler: None,
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

    pub fn enable_tls(&mut self, cert_file: PathBuf, key_file: PathBuf) {
        self.enable_tls = true;
        self.cert_file = cert_file;
        self.key_file = key_file;
    }

    pub fn set_base_url(&mut self, base_url: impl Into<String>) {
        self.base_url = base_url.into();
    }

    pub fn route_default(&mut self, handler: Box<dyn Handler>) {
        self.default_handler = Some(Arc::new(RwLock::new(handler)));
    }

    pub async fn route(&self, path: String, handler: Box<dyn Handler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push((Matcher::new(&path), handler));
    }

    pub fn start_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.start_notifier)
    }

    pub fn shutdown(&self) -> (Arc<mpsc::Sender<bool>>, Arc<Notify>) {
        (
            Arc::clone(&self.shutdown_tx),
            Arc::clone(&self.done_notifier),
        )
    }

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

        let hostport = format!("{}:{}", self.address, self.port);
        info!("Listening on {}", hostport);
        let listener = TcpListener::bind(hostport)
            .await
            .map_err(|e| e.to_string())?;
        let shutdown_notifier = Arc::new(Notify::new());

        // Let tests know we're ready
        self.start_notifier.notify_one();

        // Start keepalive proccessor background thread
        self.conn_tracker.read().await.process_keepalives().await;

        'top: loop {
            let shutdown_notifier = Arc::clone(&shutdown_notifier);
            let conn_tracker = Arc::clone(&self.conn_tracker);
            let (tcp_socket, _) = tokio::select! {
                result = listener.accept() => {
                    if let Err(err) = result {
                        // Don't propagate accept errors, just continue.
                        debug!("accept error: {}", err.to_string());
                        continue 'top;
                    }
                    result.unwrap()
                },
                _ = self.shutdown_rx.recv() => {
                    shutdown_notifier.notify_one();
                    conn_tracker.read().await.shutdown();
                    info!("Shutting down...");
                    break 'top;
                }
            };

            let socket: Box<dyn AsyncStream>;

            // If TLS
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
                socket = Box::new(tcp_socket);
            }

            let conn = self.conn_tracker.write().await.push_stream(socket);
            let base_url = self.base_url.clone();
            let handlers = Arc::clone(&self.handlers);
            let default_handler: Option<Arc<RwLock<Box<dyn Handler>>>> = self
                .default_handler
                .as_ref()
                .and_then(|h| Some(Arc::clone(&h)));

            tokio::spawn(async move {
                let mut stream = ConnectedServer {
                    conn,
                    base_url,
                    handlers,
                    default_handler,
                    shutdown_notifier,
                    conn_tracker,
                    close_connection: false,
                };

                if let Err(err) = stream.process_connection().await {
                    warn!("server error: {err}");
                    _ = stream.conn.stream().write().await.shutdown().await;
                }
            });
        }

        // Let tests know we're done
        self.done_notifier.notify_one();

        Ok(())
    }
}

#[derive(Debug)]
struct ConnectedServer {
    base_url: String,
    handlers: Arc<RwLock<Vec<(Matcher, Box<dyn Handler>)>>>,
    default_handler: Option<Arc<RwLock<Box<dyn Handler>>>>,
    conn_tracker: Arc<RwLock<ConnTracker>>,
    conn: Conn,
    shutdown_notifier: Arc<Notify>,
    close_connection: bool,
}

impl ConnectedServer {
    async fn process_headers(&mut self, headers: &HashMap<String, String>) {
        if let Some(connection) = headers.get("connection") {
            match connection.to_lowercase().as_ref() {
                "keep-alive" => {
                    if let Some(keepalive) = headers.get("keep-alive") {
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
                "close" => {
                    self.close_connection = true;
                }
                _ => {}
            }
        }
    }

    async fn process_connection(&mut self) -> Result<(), String> {
        let s1 = self.conn.stream();
        let mut s = s1.write().await;
        let timeout_notifier = self.conn.timeout_notifier();

        /*
        info!(
            "Connection ID {} received from {:?}",
            &self.conn.id(),
            s.peer_addr().unwrap()
        );
        */

        'top: loop {
            if self.close_connection {
                // We received `Connection: close`
                _ = s.shutdown().await;
                break;
            }

            let mut parser = RequestParser::new();
            parser.set_base_url(&self.base_url);

            // We're trying to keep the connection open here, and keep parsing requests until
            // the socket is closed.

            // CHUNK: is_ready()
            while !parser.is_complete() {
                let mut buf = [0u8; 16384];

                let result = tokio::select! {
                    r = s.read(&mut buf) => r,
                    _ = self.shutdown_notifier.notified() => {
                        debug!("Shutting down connection {}...", &self.conn.id());
                        break 'top;
                    }
                    _ = timeout_notifier.notified() => {
                        debug!("Keepalive timeout for connection {}...", &self.conn.id());
                        break 'top;
                    }
                };

                match result {
                    Ok(0) => {
                        // Connection closed
                        debug!("read {} bytes", 0);
                        break 'top;
                    }
                    Ok(n) => {
                        debug!("read {} bytes", n);
                        parser.parse_buf(&buf[..n]).map_err(|e| e.to_string())?;
                    }
                    Err(e) => {
                        // Socket is closed, exit this method right away.
                        debug!("connection closed: {:?}", e);
                        break 'top;
                    }
                }
            }

            // CHUNK: inc_chunk_count()
            if self.conn.inc_request_count() {
                _ = s.shutdown().await;
                break 'top;
            }

            // CHUNK: match (request vs. chunk)
            //  if chunk:
            //      response.push_chunk(...)
            //  else:
            //      follow path below
            // If we're here, then the parser has parsed a full request payload.
            let mut request: Request = parser.get_message().into();
            request.set_header("X-Hype-Connection-ID", self.conn.id().clone());
            request.set_conn(self.conn.clone());
            self.process_headers(&request.headers).await;

            debug!("Request: {:?}", request);

            let mut path = String::from("/__bad_path__");
            if let Some(url) = &request.url {
                path = url.path().into()
            }

            for handler in self.handlers.read().await.iter() {
                if let Some(matched_path) = handler.0.matches(&path) {
                    request.handler_path = Some(String::from(matched_path.to_string_lossy()));
                    if let Err(error) = handler.1.handle(&request, &mut *s).await {
                        error!("Error from handler {:?}: {:?}", handler, error);
                    }
                    continue;
                }
            }

            if let Some(handler) = &self.default_handler {
                if let Err(error) = handler.read().await.handle(&request, &mut *s).await {
                    error!("Error from handler {:?}: {:?}", handler, error);
                }
                continue;
            }

            // Fell through here, no handlers match
            let mut response = Response::new(status::from(status::NOT_FOUND));
            response.set_header("Content-Type", "text/plain");
            response.set_body("Hype: no route handlers installed.".into());
            let buf = response.serialize();
            s.write_all(buf.as_bytes())
                .await
                .map_err(|e| format!("could not write to socket: {e}"))?;
        }

        debug!("Closed connection {}", &self.conn.id());
        Ok(())
        // If we're here, then the connection is closed, there's nothing to do.
    }
}
