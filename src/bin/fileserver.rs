#[macro_use]
extern crate log;

use env_logger::Env;
use hype::server::Server;

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    info!("Starting hype...");
    let mut server = Server::new("127.0.0.1".into(), 4000);
    server.route_default(Box::new(hype::handlers::file::File::new("./tmp".into())));

    server.start().await.unwrap();
}
