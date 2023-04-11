#[macro_use]
extern crate log;

use std::sync::Arc;

use argh::FromArgs;

use async_trait::async_trait;
use hype::{
    handler::{self, AsyncWriteStream, Handler},
    handlers, lbconfig,
    request::{Method, Request},
    response::Response,
    server::Server,
    status,
};
use tokio::{io::AsyncWriteExt, sync::RwLock};

#[derive(FromArgs)]
/// Hype Load Balancer
struct Args {
    /// admin server hostname or IP
    #[argh(option, short = 'h', default = "String::from(\"localhost\")")]
    host: String,

    /// admin server port
    #[argh(option, short = 'p', default = "5000")]
    port: u16,
}

struct BackendHandler {
    _config: Arc<RwLock<lbconfig::Config>>,
    backends: Arc<RwLock<Vec<lbconfig::Backend>>>,
}

// Test with:
//   curl -d '{ "host": "foobar", "port": 3000 }' -X POST http://localhost:5000/backends
#[async_trait]
impl Handler for BackendHandler {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_body(format!("<html>Path: {}</html>\n", r.path()).into());

        match (r.method, r.path().as_str()) {
            (Method::POST, "") => {
                let backend: lbconfig::Backend = handler::parse_json(&r.body.content().await)?;
                response.set_body(format!("Got backend: {:#?}", backend).into());
                self.backends.write().await.push(backend);
            }
            _ => {
                return Err(handler::Error::Failed("Invalid request".to_string()));
            }
        }

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let mut server = Server::new(&args.host, args.port);
    info!("Starting hype admin server on {}:{}", args.host, args.port);

    let config = Arc::new(RwLock::new(lbconfig::Config::default()));

    server
        .route(
            "/backends",
            Box::new(BackendHandler {
                _config: Arc::clone(&config),
                backends: Arc::new(RwLock::new(vec![])),
            }),
        )
        .await;

    server.route_default(Box::new(handlers::status::NotFoundHandler()));
    server.start().await.unwrap();
}
