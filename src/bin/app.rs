#[macro_use]
extern crate log;

use std::sync::Arc;

use async_trait::async_trait;
use hype::{
    handler::{self, AsyncWriteStream, Handler},
    middleware,
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

struct LogHandler {}

#[async_trait]
impl Handler for LogHandler {
    async fn handle(
        &self,
        r: &Request,
        _: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        info!("Request: {:?}", r);
        Ok(handler::Ok::Next)
    }
}

struct MyHandler {
    app: Arc<Mutex<App>>,
}

impl MyHandler {
    fn new(app: Arc<Mutex<App>>) -> MyHandler {
        MyHandler { app }
    }

    async fn write_response<'a>(
        w: &mut dyn AsyncWriteStream,
        status: status::Code<'a>,
        body: String,
    ) {
        let mut response = Response::new(status::from(status));
        response.body = body.into();

        w.write_all(response.serialize().as_bytes()).await.unwrap();
    }

    async fn handle_root(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        MyHandler::write_response(w, status::OK, "<html>hi!</html>\n".into()).await;
        Ok(handler::Ok::Done)
    }

    async fn handle_get_counter(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        MyHandler::write_response(
            w,
            status::OK,
            format!("<html>count: {}</html>\n", self.app.lock().await.get()),
        )
        .await;
        Ok(handler::Ok::Done)
    }

    async fn handle_post_inc(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        self.app.lock().await.inc();
        MyHandler::write_response(w, status::OK, "{ \"op\": \"inc\" }\n".into()).await;
        Ok(handler::Ok::Done)
    }

    async fn handle_not_found(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        MyHandler::write_response(w, status::NOT_FOUND, "<html>NOT FOUND!</html>\n".into()).await;
        Ok(handler::Ok::Done)
    }
}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        match (r.method, r.path().as_str()) {
            (Method::GET | Method::POST, "/") => self.handle_root(r, w).await,
            (Method::GET, "/counter") => self.handle_get_counter(r, w).await,
            (Method::POST, "/inc") => self.handle_post_inc(r, w).await,
            _ => self.handle_not_found(r, w).await,
        }
    }
}

#[tokio::main]
async fn main() {
    hype::logger::init();

    info!("Starting hype...");
    let mut server = Server::new("127.0.0.1", 4000);
    let app = Arc::new(Mutex::new(App::new()));

    let mut stack = middleware::Stack::new();
    stack.push_handlers(&mut vec![
        Box::new(LogHandler {}),
        Box::new(MyHandler::new(app.clone())),
    ]);
    server.route_default(Box::new(stack));

    server.start().await.unwrap();
}
