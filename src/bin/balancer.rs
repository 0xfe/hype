#[macro_use]
extern crate log;

use std::fs;

use argh::FromArgs;

use hype::{
    lb::{
        backend::{Backend, HttpBackend},
        http::Http,
        picker::RRPicker,
    },
    lbconfig::{self},
    server::Server,
};

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// server port
    #[argh(option, short = 'c', default = "String::from(\"lbconfig.yaml\")")]
    config: String,
}

fn build_backend(backend: &lbconfig::Backend) -> HttpBackend {
    let mut b = HttpBackend::new(format!("{}:{}", backend.host, backend.port));
    if backend.enable_tls {
        b.enable_tls(backend.host.clone());
    }
    b
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let input = fs::read_to_string(args.config).unwrap();
    let config = lbconfig::Config::from_str(input).unwrap();
    debug!("Config: {:?}", config);

    let mut server = Server::new(config.server.listen_ip, config.server.port);
    if config.server.enable_tls {
        server.enable_tls(
            config.server.tls_cert_file.into(),
            config.server.tls_key_file.into(),
        );
    }

    for route in config.routes {
        let backends: Vec<HttpBackend> = route.backends.iter().map(build_backend).collect();

        let mut balancer = Http::new(backends, RRPicker::new());
        if let Some(host_header) = route.host_header {
            balancer.rewrite_header("host", host_header);
        }

        let lb = hype::handlers::lb::Lb::new(balancer);
        server.route(route.location, lb).await;
    }
    server.start().await.unwrap();
}
