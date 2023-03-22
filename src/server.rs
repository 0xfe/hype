#![allow(non_snake_case)]

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::{mpsc, Notify, RwLock},
};

use crate::{
    conntrack::{Conn, ConnTracker},
    handler::Handler,
    parser::{self, Parser},
    request::Request,
    response::Response,
    router::Matcher,
    status,
};

use std::sync::Arc;

#[derive(Debug)]
struct Stream {
    base_url: String,
    handlers: Arc<RwLock<Vec<(Matcher, Box<dyn Handler>)>>>,
    default_handler: Option<Arc<RwLock<Box<dyn Handler>>>>,
    conn: Conn,
    shutdown_notifier: Arc<Notify>,
}

#[derive(Debug)]
pub struct Server {
    address: String,
    port: u16,
    base_url: String,
    handlers: Arc<RwLock<Vec<(Matcher, Box<dyn Handler>)>>>,
    default_handler: Option<Arc<RwLock<Box<dyn Handler>>>>,
    conns: Arc<RwLock<ConnTracker>>,
    start_notifier: Arc<Notify>,
    done_notifier: Arc<Notify>,
    shutdown_tx: Arc<mpsc::Sender<bool>>,
    shutdown_rx: mpsc::Receiver<bool>,
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
            conns: Arc::new(RwLock::new(ConnTracker::new())),
            start_notifier: Arc::new(Notify::new()),
            done_notifier: Arc::new(Notify::new()),
            shutdown_tx: Arc::new(tx),
            shutdown_rx: rx,
        }
    }

    async fn process_stream(stream: Stream) {
        let s1 = stream.conn.stream();
        let mut s = s1.write().await;

        info!(
            "Connection ID {} received from {:?}",
            &stream.conn.id(),
            s.peer_addr().unwrap()
        );

        'top: loop {
            let mut parser = Parser::new(&stream.base_url, parser::State::StartRequest);

            // We're trying to keep the connection open here, and keep parsing requests until
            // the socket is closed.
            while !parser.is_complete() {
                let mut buf = [0u8; 16384];

                let result = tokio::select! {
                    r = s.read(&mut buf) => r,
                    _ = stream.shutdown_notifier.notified() => {
                        info!("Shutting down connection {}...", &stream.conn.id());
                        break 'top;
                    }
                };

                match result {
                    Ok(0) => {
                        // No data read, but it's possible the socket is still open.
                        debug!("read {} bytes", 0);
                        break 'top;
                    }
                    Ok(n) => {
                        debug!("read {} bytes", n);
                        parser.parse_buf(&buf[..n]).unwrap();
                    }
                    Err(e) => {
                        // Socket is closed, exit this method right away.
                        debug!("connection closed: {:?}", e);
                        break 'top;
                    }
                }
            }

            // If we're here, then the parser has parsed a full request payload.
            let mut request: Request = parser.get_message().into();
            request.set_header("X-Hype-Connection-ID", stream.conn.id().clone());
            request.set_conn(stream.conn.clone());
            debug!("Request: {:?}", request);

            let mut path = String::from("/__bad_path__");
            if let Some(url) = &request.url {
                path = url.path().into()
            }

            for handler in stream.handlers.write().await.iter_mut() {
                if let Some(matched_path) = handler.0.matches(&path) {
                    request.set_handler_path(String::from(matched_path.to_string_lossy()));
                    if let Err(error) = handler.1.handle(&request, &mut *s).await {
                        error!("Error from handler {:?}: {:?}", handler, error);
                    }
                    continue;
                }
            }

            if let Some(handler) = &stream.default_handler {
                if let Err(error) = handler.write().await.handle(&request, &mut *s).await {
                    error!("Error from handler {:?}: {:?}", handler, error);
                }
                continue;
            }

            // Fell through here, no handlers match
            let mut response = Response::new(status::from(status::NOT_FOUND));
            response.set_header("Content-Type", "text/plain");
            response.set_body("Hype: no route handlers installed.".into());
            let buf = response.serialize();
            s.write_all(buf.as_bytes()).await.unwrap();
        }

        info!("Closed connection {}", &stream.conn.id());
        // If we're here, then the connection is closed, there's nothing to do.
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

    pub async fn start(&mut self) -> Result<(), ()> {
        let hostport = format!("{}:{}", self.address, self.port);
        info!("Listening on {}", hostport);
        let listener = TcpListener::bind(hostport).await.unwrap();
        let shutdown_notifier = Arc::new(Notify::new());

        // Let tests know we're ready
        self.start_notifier.notify_one();

        'top: loop {
            let shutdown_notifier = Arc::clone(&shutdown_notifier);
            let result = tokio::select! {
                result = listener.accept() => { result.unwrap() },
                _ = self.shutdown_rx.recv() => {
                    shutdown_notifier.notify_one();
                    info!("Shutting down...");
                    break 'top;
                }
            };

            // let (socket, _) = listener.accept().await.unwrap();
            let (socket, _) = result;
            let conn = self.conns.write().await.push_stream(socket).await;

            let base_url = self.base_url.clone();
            let handlers = Arc::clone(&self.handlers);
            let default_handler: Option<Arc<RwLock<Box<dyn Handler>>>> = self
                .default_handler
                .as_ref()
                .and_then(|h| Some(Arc::clone(&h)));

            tokio::spawn(async move {
                Server::process_stream(Stream {
                    conn,
                    base_url,
                    handlers,
                    default_handler,
                    shutdown_notifier,
                })
                .await;
            });
        }

        self.done_notifier.notify_one();

        Ok(())
    }
}
