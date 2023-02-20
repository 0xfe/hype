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
    status,
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

    async fn handle_get(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        let mut response = Response::new(status::from(status::OK));

        match &r.path[..] {
            "/" => {
                response.set_body("<html>hi!</html>\n".into());
            }
            "/count" => {
                response.set_body(format!(
                    "<html>count: {}</html>\n",
                    self.app.lock().await.get()
                ));
            }
            _ => {
                response.set_status(status::from(status::NOT_FOUND));
                response.set_body("<html>404 NOT FOUND</html>".into());
            }
        }

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
    }

    async fn handle_post(
        &self,
        r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<(), handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        match &r.path[..] {
            "/" => {
                response.set_body(format!("{}\n", r.body));
            }
            "/inc" => {
                self.app.lock().await.inc();
                response.set_body("{ \"op\": \"inc\" }\n".into());
            }
            _ => {
                response.set_status(status::from(status::NOT_FOUND));
                response.set_body("<html>404 NOT FOUND</html>".into());
            }
        }

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
    }
}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        match r.method {
            Method::GET => self.handle_get(r, w).await,
            Method::POST => self.handle_post(r, w).await,
            _ => Ok(()),
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
    let server = Server::new("127.0.0.1".into(), 4000);

    let app = Arc::new(Mutex::new(App::new()));

    server
        .route("/".to_string(), Box::new(MyHandler::new(app.clone())))
        .await;
    server
        .route("/inc".to_string(), Box::new(MyHandler::new(app.clone())))
        .await;
    server
        .route("/count".to_string(), Box::new(MyHandler::new(app.clone())))
        .await;

    server.start().await.unwrap();
}
