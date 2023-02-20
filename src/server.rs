#![allow(non_snake_case)]

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{
    handler::{AsyncStream, Handler},
    request::Parser,
    response::Response,
    status,
};

use std::{collections::HashMap, sync::Arc};

impl AsyncStream for TcpStream {}

#[allow(dead_code)]
#[derive(Debug)]
enum Error {
    ConnectionBroken,
}

#[derive(Debug)]
pub struct Server {
    address: String,
    port: u16,
    base_url: String,
    handlers: Arc<RwLock<HashMap<String, Box<dyn Handler>>>>,
    default_handler: Option<Arc<Box<dyn Handler>>>,
}

impl Server {
    async fn process_stream(
        mut stream: TcpStream,
        base_url: String,
        handlers: Arc<RwLock<HashMap<String, Box<dyn Handler>>>>,
        default_handler: Option<Arc<Box<dyn Handler>>>,
    ) {
        info!("Connection received from {:?}", stream.peer_addr().unwrap());
        let mut parser = Parser::new(base_url);

        loop {
            let mut buf = [0u8; 16];

            match stream.read(&mut buf).await {
                Ok(0) => {
                    parser.parse_eof().unwrap();
                    break;
                }
                Ok(n) => {
                    parser.parse_buf(&buf[..n]).unwrap();
                    if parser.is_complete() {
                        parser.parse_eof().unwrap();
                        break;
                    }
                }
                Err(e) => {
                    warn!("Connection broken: {:?}", e);
                    break;
                }
            }
        }

        let request = parser.get_request();
        debug!("Request: {:?}", request);

        let mut path = String::from("/__bad_path__");
        if let Some(url) = &request.url {
            path = url.path().into()
        }

        if let Some(handler) = handlers.read().await.get(&path) {
            handler.handle(&request, &mut stream).await.unwrap();
        } else if let Some(handler) = default_handler {
            handler.handle(&request, &mut stream).await.unwrap();
        } else {
            let mut response = Response::new(status::from(status::NOT_FOUND));
            response.set_header("Content-Type".into(), "text/plain".into());
            response.set_body("Hype: no route handlers installed.".into());
            let buf = response.serialize();
            stream.write_all(buf.as_bytes()).await.unwrap();
        }
    }

    pub fn new(address: String, port: u16) -> Self {
        let base_url = format!("http://{}:{}", address, port);

        Self {
            address,
            port,
            handlers: Arc::new(RwLock::new(HashMap::new())),
            default_handler: None,
            base_url,
        }
    }

    pub fn route_default(&mut self, handler: Box<dyn Handler>) {
        self.default_handler = Some(Arc::new(handler));
    }

    pub async fn route(&self, path: String, handler: Box<dyn Handler>) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(path, handler);
    }

    pub async fn start(&self) -> Result<(), ()> {
        let hostport = format!("{}:{}", self.address, self.port);
        info!("Listening on {}", hostport);
        let listener = TcpListener::bind(hostport).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let base_url = self.base_url.clone();
            let handlers = Arc::clone(&self.handlers);
            let default_handler: Option<Arc<Box<dyn Handler>>> = self
                .default_handler
                .as_ref()
                .and_then(|h| Some(Arc::clone(&h)));

            tokio::spawn(async move {
                Server::process_stream(socket, base_url, handlers, default_handler).await;
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
