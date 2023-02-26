#[macro_use]
extern crate log;

use std::{env, process::exit};

use async_trait::async_trait;
use env_logger::Env;
use hype::{
    handler::{self, Handler},
    server::Server,
    status,
};
use tokio::io::AsyncWriteExt;

struct ErrorPage {}

#[async_trait]
impl Handler for ErrorPage {
    async fn handle(
        &self,
        _r: &hype::request::Request,
        w: &mut dyn hype::handler::AsyncStream,
    ) -> Result<(), handler::Error> {
        let mut response = hype::response::Response::new(status::from(status::OK));
        response.set_body("<html>404 NOT FOUND</html>\n".into());
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

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        error!("Usage: fileserver [path]");
        exit(255);
    }

    info!("Starting hype:fileserver at path '{}'", args[1]);
    let mut server = Server::new("127.0.0.1".into(), 4000);
    server
        .route(
            "/files".to_string(),
            Box::new(hype::handlers::file::File::new(args[1].clone())),
        )
        .await;
    server.route_default(Box::new(ErrorPage {}));

    server.start().await.unwrap();
}
