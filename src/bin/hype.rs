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
    server::Server,
    status,
};
use tokio::sync::RwLock;

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

// Test with:
//   curl -d '{ "host": "foobar", "port": 3000 }'  -H "x-hype-auth-token: foo" -X POST http://localhost:5000/backends
//   curl -H "x-hype-auth-token: foo" http://localhost:5000/backends/backend-ayGoPVg

#[derive(Debug, Clone)]
struct State {
    backends: Arc<RwLock<HashMap<String, lbconfig::Backend>>>,
}

async fn add_backend(
    _: Request,
    backend: lbconfig::Backend,
    state: State,
) -> (status::Status, String) {
    let id = backend.id.clone();
    state.backends.write().await.insert(id.clone(), backend);
    (status::from(status::OK), format!("Got backend: {:?}", id))
}

async fn get_backend(r: Request, state: State) -> Result<(status::Status, String), handler::Error> {
    let response = format!(
        "{:#?}",
        state
            .backends
            .read()
            .await
            .get(
                r.params
                    .get("id")
                    .ok_or(handler::Error::Failed("missing parameter: id".to_string()))?
            )
            .ok_or(handler::Error::Status(status::from(status::NOT_FOUND)))?
    )
    .into();
    Ok((status::from(status::OK), response))
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let mut server = Server::new(&args.host, args.port);
    info!("Starting hype admin server on {}:{}", args.host, args.port);

    let auth_handler = AuthHandler {
        token: "foo".into(),
    };

    let state = State {
        backends: Arc::new(RwLock::new(HashMap::new())),
    };

    let mut add_stack = Stack::new();
    add_stack.push(handlers::log::Log {});
    add_stack.push(auth_handler.clone());
    add_stack.push(handler::json(add_backend, state.clone()));
    server
        .route_method(Method::POST, "/backends", add_stack)
        .await;

    let mut get_stack = Stack::new();
    get_stack.push(handlers::log::Log {});
    get_stack.push(auth_handler);
    get_stack.push(handler::get(get_backend, state.clone()));
    server.route("/backends/:id", get_stack).await;

    server.route_default(handlers::status::NotFoundHandler());
    server.start().await.unwrap();
}
