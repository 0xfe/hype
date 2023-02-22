#[macro_use]
extern crate log;

use std::{env, process::exit};

use env_logger::Env;
use hype::server::Server;

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
    server.route_default(Box::new(hype::handlers::file::File::new(args[1].clone())));

    server.start().await.unwrap();
}
