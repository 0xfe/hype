#![allow(non_snake_case)]

use futures::future::poll_fn;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, ReadBuf},
    net::TcpListener,
    sync::RwLock,
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

#[allow(dead_code)]
#[derive(Debug)]
enum Error {
    ConnectionBroken,
}

#[derive(Debug)]
struct Stream {
    base_url: String,
    handlers: Arc<RwLock<Vec<(Matcher, Box<dyn Handler>)>>>,
    default_handler: Option<Arc<RwLock<Box<dyn Handler>>>>,
    conn: Conn,
}

#[derive(Debug)]
pub struct Server {
    address: String,
    port: u16,
    base_url: String,
    handlers: Arc<RwLock<Vec<(Matcher, Box<dyn Handler>)>>>,
    default_handler: Option<Arc<RwLock<Box<dyn Handler>>>>,
    conns: Arc<RwLock<ConnTracker>>,
}

impl Server {
    pub fn new<T: Into<String> + Clone>(address: T, port: u16) -> Self {
        let base_url = format!("http://{}:{}", address.clone().into(), port);

        Self {
            address: address.into(),
            port,
            handlers: Arc::new(RwLock::new(Vec::new())),
            default_handler: None,
            base_url,
            conns: Arc::new(RwLock::new(ConnTracker::new())),
        }
    }

    async fn process_stream(stream: Stream) {
        let s1 = stream.conn.stream();
        let mut s = s1.write().await;

        // These are used for poll_peek below. They probably could just be a byte.
        let mut poll_buf = [0u8; 10];
        let mut poll_buf = ReadBuf::new(&mut poll_buf);

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

                // This blocks until the socket is readable. The peek() or ready() don't block. We
                // need poll_fn to get a Context type (cx).
                if let Err(e) = poll_fn(|cx| s.poll_peek(cx, &mut poll_buf)).await {
                    debug!("connection closed: {:?}", e);
                    break 'top;
                }

                match s.read(&mut buf).await {
                    Ok(0) => {
                        // No data read, but it's possible the socket is still open.
                        debug!("read {} bytes", 0);
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
            request.push_header("X-Hype-Connection-ID", stream.conn.id().clone());
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

        // If we're here, then the connection is closed, there's nothing to do.
    }

    pub fn route_default(&mut self, handler: Box<dyn Handler>) {
        self.default_handler = Some(Arc::new(RwLock::new(handler)));
    }

    pub async fn route(&self, path: String, handler: Box<dyn Handler>) {
        let mut handlers = self.handlers.write().await;
        handlers.push((Matcher::new(&path), handler));
    }

    pub async fn start(&mut self) -> Result<(), ()> {
        let hostport = format!("{}:{}", self.address, self.port);
        info!("Listening on {}", hostport);
        let listener = TcpListener::bind(hostport).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();
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
                })
                .await;
            });
        }
    }
}
