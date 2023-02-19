#[macro_use]
extern crate log;

use async_trait::async_trait;
use env_logger::Env;
use hype::{
    handler::{self, AsyncStream, Handler},
    parser::Request,
    response::Response,
    server::Server,
    status,
};
use tokio::io::AsyncWriteExt;

struct MyHandler {}

impl MyHandler {
    async fn handle_get(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        let mut response = Response::new(status::from(status::OK));

        match &r.path[..] {
            "/" => {
                response.set_body("<html>hi!</html>\n".into());
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
        response.set_body(format!("{}\n", r.body));
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
    }
}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        match &r.method[..] {
            "GET" => self.handle_get(r, w).await,
            "POST" => self.handle_post(r, w).await,
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

    server.route("/".to_string(), Box::new(MyHandler {})).await;
    server
        .route("/foo".to_string(), Box::new(MyHandler {}))
        .await;

    server.start().await.unwrap();
}
