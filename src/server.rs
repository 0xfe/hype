#![allow(non_snake_case)]

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::RwLock,
};

use crate::{
    handler::{AsyncStream, Handler},
    parser::{Parser, Request},
    response::Response,
    status,
};

use std::{collections::HashMap, str, sync::Arc};

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
    handlers: Arc<RwLock<HashMap<String, Handler>>>,
}

impl Server {
    async fn process_GET(request: Request, stream: &mut TcpStream) -> Result<(), Error> {
        let mut response = Response::new(status::from(status::OK));

        match &request.path[..] {
            "/" => {
                response.set_body("<html>hi!</html>\n".into());
            }
            _ => {
                response.set_status(status::from(status::NOT_FOUND));
                response.set_body("<html>404 NOT FOUND</html>".into());
            }
        }

        let buf = response.serialize();
        stream.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
    }

    async fn process_POST(request: Request, stream: &mut TcpStream) -> Result<(), Error> {
        let mut response = Response::new(status::from(status::OK));

        match &request.path[..] {
            "/" => {
                response.set_body(format!(
                    "{{\"request\": {}}}\n",
                    str::from_utf8(&request.body[..]).unwrap()
                ));
            }
            _ => {
                response.set_status(status::from(status::NOT_FOUND));
                response.set_body("<html>404 NOT FOUND</html>".into());
            }
        }

        let buf = response.serialize();
        stream.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
    }

    async fn process_stream(
        mut stream: TcpStream,
        handlers: Arc<RwLock<HashMap<String, Handler>>>,
    ) {
        info!("Connection received from {:?}", stream.peer_addr().unwrap());

        info!("Handlers: {:?}", handlers.read().await);
        let mut parser = Parser::new();

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

        if let Some(handler) = handlers.read().await.get(&request.path) {
            handler.call(&request, &mut stream).await.unwrap();
        }

        match &request.method[..] {
            "GET" => Server::process_GET(request, &mut stream).await.unwrap(),
            "POST" => Server::process_POST(request, &mut stream).await.unwrap(),
            _ => {
                let mut response = Response::new(status::from(status::SERVER_ERROR));
                response.set_body("<html>boo!</html>\n".into());
                let buf = response.serialize();

                stream.write_all(buf.as_bytes()).await.unwrap();
            }
        }
    }

    pub fn new(address: String, port: u16) -> Self {
        Self {
            address,
            port,
            handlers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn handle(&self, path: String, handler: Handler) {
        let mut handlers = self.handlers.write().await;
        handlers.insert(path, handler);
    }

    pub async fn start(&self) -> Result<(), ()> {
        let hostport = format!("{}:{}", self.address, self.port);
        info!("Listening on {}", hostport);
        let listener = TcpListener::bind(hostport).await.unwrap();

        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let handlers = Arc::clone(&self.handlers);
            tokio::spawn(async move {
                Server::process_stream(socket, handlers).await;
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
