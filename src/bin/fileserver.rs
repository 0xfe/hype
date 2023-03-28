#[macro_use]
extern crate log;

use std::{env, process::exit};

use hype::{handlers, server::Server};

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    hype::logger::init();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        error!("Usage: fileserver [path]");
        exit(255);
    }

    info!("Starting hype:fileserver at path '{}'", args[1]);
    let mut server = Server::new("127.0.0.1", 4000);
    server
        .route(
            "/files".to_string(),
            Box::new(hype::handlers::file::File::new(args[1].clone())),
        )
        .await;
    server.route_default(Box::new(handlers::status::NotFoundHandler()));

    server.start().await.unwrap();
}
