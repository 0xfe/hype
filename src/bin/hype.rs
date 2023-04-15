#[macro_use]
extern crate log;

use std::{collections::HashMap, sync::Arc};

use argh::FromArgs;

use async_trait::async_trait;
use hype::{
    handler::{self, AsyncWriteStream, Handler},
    handlers, lbconfig,
    middleware::Stack,
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

#[derive(Clone, Debug)]
struct AuthHandler {
    token: String,
}

#[async_trait]
impl Handler for AuthHandler {
    async fn handle(
        &self,
        r: &Request,
        _w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Action, handler::Error> {
        let token = r
            .headers
            .get_first("x-hype-auth-token")
            .ok_or(handler::Error::Status(status::from(status::UNAUTHORIZED)))?;

        if *token != self.token {
            return Err(handler::Error::Status(status::from(status::UNAUTHORIZED)));
        }

        Ok(handler::Action::Next)
    }
}

struct BackendHandler {
    _config: Arc<RwLock<lbconfig::Config>>,
    backends: Arc<RwLock<HashMap<String, lbconfig::Backend>>>,
}

// Test with:
//   curl -d '{ "host": "foobar", "port": 3000 }'  -H "x-hype-auth-token: foo" -X POST http://localhost:5000/backends
//   curl -H "x-hype-auth-token: foo" http://localhost:5000/backends/backend-ayGoPVg
#[async_trait]
impl Handler for BackendHandler {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Action, handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_body(format!("<html>Path: {}</html>\n", r.path()).into());

        match (r.method, r.path().as_str()) {
            (Method::POST, _) => {
                let backend: lbconfig::Backend = handler::parse_json(&r.body.content().await)?;
                response.set_body(format!("Got backend: {:#?}", &backend).into());
                self.backends
                    .write()
                    .await
                    .insert(backend.id.clone(), backend);
            }
            (Method::GET, _) => {
                response.set_body(
                    format!(
                        "{:#?}",
                        self.backends
                            .read()
                            .await
                            .get(r.params.get("id").ok_or(handler::Error::Failed(
                                "missing parameter: id".to_string()
                            ))?)
                            .ok_or(handler::Error::Status(status::from(status::NOT_FOUND)))?
                    )
                    .into(),
                );
            }
            _ => {
                return Err(handler::Error::Failed("Invalid request".to_string()));
            }
        }

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Action::Done)
    }
}

async fn add_backend(_r: Request, backend: lbconfig::Backend) -> (status::Status, String) {
    (
        status::from(status::OK),
        format!("Got backend: {:#?}", &backend),
    )
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let mut server = Server::new(&args.host, args.port);
    info!("Starting hype admin server on {}:{}", args.host, args.port);

    let config = Arc::new(RwLock::new(lbconfig::Config::default()));
    let backends = Arc::new(RwLock::new(HashMap::new()));

    let auth_handler = AuthHandler {
        token: "foo".into(),
    };

    let mut stack = Stack::new();
    stack.push(handlers::log::Log {});
    stack.push(auth_handler);
    stack.push(BackendHandler {
        _config: Arc::clone(&config),
        backends: Arc::clone(&backends),
    });

    server
        .route("/test_add_backend", handler::post(add_backend))
        .await;
    server.route("/backends", stack.clone()).await;
    server.route("/backends/:id", stack.clone()).await;

    server.route_default(handlers::status::NotFoundHandler());
    server.start().await.unwrap();
}
