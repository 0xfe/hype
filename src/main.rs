#[macro_use]
extern crate log;

use env_logger::Env;
use hype::{handler, server::Server};

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting hype...");
    let server = Server::new("127.0.0.1".into(), 4000);

    server.handle(
        "/".to_string(),
        handler::Handler::new(
            "GET".to_string(),
            Box::new(|request| -> Result<(), handler::Error> {
                println!("{:?}", request);
                Ok(())
            }),
        ),
    );

    server.start().await.unwrap();
}
