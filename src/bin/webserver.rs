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
    router::RouteHandler,
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
    ) -> Result<handler::Action, handler::Error> {
        let mut response = Response::new(status::from(status::NOT_FOUND));
        response.set_body(format!("404 File not found: {}\n", r.path()));
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Action::Done)
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
        let handler: RouteHandler = match &route.handler {
            config::Handler::File(params) => {
                handlers::file::File::new(params.fs_path.clone()).into()
            }
            config::Handler::Web(params) => handlers::web::Web::from(params).into(),
        };

        server.route(route.location.clone(), handler).await;
    }

    server.route_default(MyHandler {});
    server.start().await.unwrap();
}
