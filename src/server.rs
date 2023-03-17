#![allow(non_snake_case)]

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::RwLock,
};

use crate::{
    conntrack::{ConnId, ConnTracker},
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
    conns: Arc<RwLock<ConnTracker>>,
    conn_id: ConnId,
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
    pub fn new(address: String, port: u16) -> Self {
        let base_url = format!("http://{}:{}", address, port);

        Self {
            address,
            port,
            handlers: Arc::new(RwLock::new(Vec::new())),
            default_handler: None,
            base_url,
            conns: Arc::new(RwLock::new(ConnTracker::new())),
        }
    }

    async fn process_stream(stream: Stream) {
        let s1 = stream
            .conns
            .read()
            .await
            .stream(&stream.conn_id)
            .await
            .unwrap();

        let mut s = s1.write().await;

        info!(
            "Connection ID {} received from {:?}",
            &stream.conn_id,
            s.peer_addr().unwrap()
        );

        let mut done = false;

        while !done {
            let mut parser = Parser::new(&stream.base_url, parser::State::StartRequest);
            done = loop {
                let mut buf = [0u8; 16384];

                match s.read(&mut buf).await {
                    Ok(0) => {
                        debug!("read {} bytes", 0);
                        parser.parse_eof().unwrap();
                        break false;
                    }
                    Ok(n) => {
                        debug!("read {} bytes", n);
                        parser.parse_buf(&buf[..n]).unwrap();

                        // Clients may leave the connection open, so check to see if we've
                        // got a full request in. (Otherwise, we just block.)
                        if parser.is_complete() {
                            debug!("request received, keeping client connection open");
                            parser.parse_eof().unwrap();
                            break false;
                        }
                    }
                    Err(e) => {
                        debug!("connection closed by client");
                        warn!("Connection broken: {:?}", e);
                        break true;
                    }
                }
            };

            let mut request: Request = parser.get_message().into();
            request.push_header("X-Hype-Connection-ID", stream.conn_id.clone());
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
            let conns = Arc::clone(&self.conns);
            let id = conns.write().await.push_stream(socket).await;

            let base_url = self.base_url.clone();
            let handlers = Arc::clone(&self.handlers);
            let default_handler: Option<Arc<RwLock<Box<dyn Handler>>>> = self
                .default_handler
                .as_ref()
                .and_then(|h| Some(Arc::clone(&h)));

            tokio::spawn(async move {
                Server::process_stream(Stream {
                    conns: conns.clone(),
                    conn_id: id,
                    base_url,
                    handlers,
                    default_handler,
                })
                .await;
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let server = Server::new("a".into(), 10);
        assert_eq!(server.port, 10);
    }
}
