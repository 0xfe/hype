#[macro_use]
extern crate log;

use async_trait::async_trait;
use env_logger::Env;
use hype::{
    handler::{self, AsyncStream, HandlerFn},
    parser::Request,
    response::Response,
    server::Server,
    status,
};
use tokio::io::AsyncWriteExt;

struct MyHandler {}

#[async_trait]
impl HandlerFn for MyHandler {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        println!("{:?}", r);
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
}

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting hype...");
    let server = Server::new("127.0.0.1".into(), 4000);

    server
        .handle(
            "/".to_string(),
            handler::Handler::new("GET".to_string(), Box::new(MyHandler {})),
        )
        .await;

    server.start().await.unwrap();
}
