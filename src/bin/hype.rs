#[macro_use]
extern crate log;

// Test with:
//   curl -d '{ "host": "foobar", "port": 3000 }'  -H "x-hype-auth-token: foo" -X POST http://localhost:5000/backends
//   curl -H "x-hype-auth-token: foo" http://localhost:5000/backends/backend-ayGoPVg

use std::{collections::HashMap, sync::Arc};

use argh::FromArgs;

use hype::{
    handler::{self, Action},
    handlers::{self},
    lbconfig,
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

#[derive(Clone, Debug, Default)]
struct AuthState {
    token: String,
}

async fn auth(r: Request, state: AuthState) -> Result<Action, handler::Error> {
    let token = r
        .headers
        .get_first("x-hype-auth-token")
        .ok_or(handler::Error::Status(status::from(status::UNAUTHORIZED)))?;

    if *token != state.token {
        return Err(handler::Error::Status(status::from(status::UNAUTHORIZED)));
    }

    // Don't return a response as yet, simply pass the request on to the next handler.
    Ok(Action::Next)
}

#[derive(Debug, Clone, Default)]
struct AppState {
    backends: Arc<RwLock<HashMap<String, lbconfig::Backend>>>,
}

async fn add_backend(r: Request, state: AppState) -> Result<String, handler::Error> {
    let backend: lbconfig::Backend = handlers::service::json(&r.body.content().await)?;
    let id = backend.id.clone();
    state.backends.write().await.insert(id.clone(), backend);
    Ok(format!("Got backend: {:?}", id))
}

async fn get_backend(r: Request, state: AppState) -> Result<String, handler::Error> {
    let id = r
        .params
        .get("id")
        .ok_or(handler::Error::Failed("missing parameter: id".to_string()))?;

    let lock = state.backends.read().await;
    let backend = lock
        .get(id)
        .ok_or(handler::Error::Status(status::from(status::NOT_FOUND)))?;

    Ok(format!("{:#?}", backend))
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let mut server = Server::new(&args.host, args.port);
    info!("Starting hype admin server on {}:{}", args.host, args.port);

    let middleware = Stack::new()
        .push(handlers::log())
        .push(handlers::service(auth).with_state(AuthState {
            token: "foo".to_string(),
        }));

    let state = AppState {
        backends: Arc::new(RwLock::new(HashMap::new())),
    };

    server.route_method(
        Method::POST,
        "/backends",
        middleware
            .clone()
            .push(handlers::service(add_backend).with_state(state.clone())),
    );

    server.route_method(
        Method::GET,
        "/backends/:id",
        middleware
            .clone()
            .push(handlers::service(get_backend).with_state(state.clone())),
    );

    server.route_default(handlers::NotFoundHandler());
    server.start().await.unwrap();
}
