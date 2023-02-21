#[macro_use]
extern crate log;

use async_trait::async_trait;
use env_logger::Env;
use hype::{
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    server::Server,
    status::{self},
};
use tokio::io::AsyncWriteExt;

struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&self, _r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_body("<html>Hello world!</html>\n".into());
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
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
    server.route_default(Box::new(MyHandler {}));

    server.start().await.unwrap();
}
