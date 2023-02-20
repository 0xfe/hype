#[macro_use]
extern crate log;

use std::sync::Arc;

use async_trait::async_trait;
use env_logger::Env;
use hype::{
    handler::{self, AsyncStream, Handler},
    request::{Method, Request},
    response::Response,
    server::Server,
    status::{self},
};
use tokio::{io::AsyncWriteExt, sync::Mutex};

struct App {
    counter: u32,
}

impl App {
    fn new() -> App {
        App { counter: 0 }
    }

    fn inc(&mut self) {
        self.counter += 1
    }

    fn get(&self) -> u32 {
        self.counter
    }
}

struct MyHandler {
    app: Arc<Mutex<App>>,
}

impl MyHandler {
    fn new(app: Arc<Mutex<App>>) -> MyHandler {
        MyHandler { app }
    }

    async fn write_response<'a>(w: &mut dyn AsyncStream, status: status::Code<'a>, body: String) {
        let mut response = Response::new(status::from(status));
        response.set_body(body);
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
    }

    async fn handle_root(
        &self,
        _r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<(), handler::Error> {
        MyHandler::write_response(w, status::OK, "<html>hi!</html>\n".into()).await;
        Ok(())
    }

    async fn handle_get_counter(
        &self,
        _r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<(), handler::Error> {
        MyHandler::write_response(
            w,
            status::OK,
            format!("<html>count: {}</html>\n", self.app.lock().await.get()),
        )
        .await;
        Ok(())
    }

    async fn handle_post_inc(
        &self,
        _r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<(), handler::Error> {
        self.app.lock().await.inc();
        MyHandler::write_response(w, status::OK, "{ \"op\": \"inc\" }\n".into()).await;
        Ok(())
    }

    async fn handle_not_found(
        &self,
        _r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<(), handler::Error> {
        MyHandler::write_response(w, status::NOT_FOUND, "<html>NOT FOUND!</html>\n".into()).await;
        Ok(())
    }
}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        match (r.method, &r.path[..]) {
            (Method::GET | Method::POST, "/") => self.handle_root(r, w).await,
            (Method::GET, "/counter") => self.handle_get_counter(r, w).await,
            (Method::POST, "/inc") => self.handle_post_inc(r, w).await,
            _ => self.handle_not_found(r, w).await,
        }
    }
}

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting hype...");
    let mut server = Server::new("127.0.0.1".into(), 4000);
    let app = Arc::new(Mutex::new(App::new()));
    server.route_default(Box::new(MyHandler::new(app.clone())));

    server.start().await.unwrap();
}
