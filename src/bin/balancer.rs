#[macro_use]
extern crate log;

use std::fs;

use argh::FromArgs;

use hype::{
    handlers,
    lb::{backend::HttpBackend, http::Http, picker::RRPicker},
    lbconfig,
    server::Server,
};

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// server port
    #[argh(option, short = 'c', default = "String::from(\"lbconfig.yaml\")")]
    config: String,
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let input = fs::read_to_string(args.config).unwrap();
    let config = lbconfig::Config::from_str(input).unwrap();
    debug!("Config: {:?}", config);

    let mut server = Server::new(config.server.listen_ip, config.server.port);

    for route in config.routes {
        let backends: Vec<HttpBackend> =
            route.backends.iter().map(|b| HttpBackend::new(b)).collect();

        let mut balancer = Http::new(backends, RRPicker::new());
        balancer.rewrite_header("host", "google.com");

        let lb = hype::handlers::lb::Lb::new(balancer);
        server.route(route.location, Box::new(lb)).await;
    }

    server.route_default(Box::new(handlers::status::NotFoundHandler()));
    server.start().await.unwrap();
}
