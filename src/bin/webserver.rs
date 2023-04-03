#[macro_use]
extern crate log;

use std::fs;

use async_trait::async_trait;
use hype::{
    config::{self, Config},
    handler::{self, AsyncWriteStream, Handler},
    handlers,
    request::Request,
    response::Response,
    server::Server,
    status,
};

use tokio::io::AsyncWriteExt;

struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::NOT_FOUND));
        response.set_body(format!("404 File not found: {}\n", r.path()).into());
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

#[tokio::main]
async fn main() {
    hype::logger::init();

    let args: Vec<String> = std::env::args().collect();

    if args.len() < 2 {
        error!("Usage: webserver [/path/to/config.yaml]");
        std::process::exit(255);
    }

    let input = fs::read_to_string(&args[1]).unwrap();
    let config = Config::from_str(input).expect("bad configuration file");

    info!("Starting hype...");
    debug!("config: {:?}", config);

    let mut server = Server::new(config.server.listen_ip, config.server.port);

    for route in &config.routes {
        let handler: Box<dyn Handler> = match &route.handler {
            config::Handler::File(params) => {
                Box::new(handlers::file::File::new(params.fs_path.clone()))
            }
            config::Handler::Web(params) => Box::new(handlers::web::Web::from(params)),
        };

        server.route(route.location.clone(), handler).await;
    }

    server.route_default(Box::new(MyHandler {}));
    server.start().await.unwrap();
}
