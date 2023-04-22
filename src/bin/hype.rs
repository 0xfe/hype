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
    lbconfig::{self, BackendId, RouteId},
    middleware::Stack,
    request::{Method, Request},
    server::Server,
    status,
};
use serde::Deserialize;
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

/// A very basic auth mechanism used for development.
#[derive(Clone, Debug, Default)]
struct AuthState {
    token: String,
}

async fn auth(r: Request, state: AuthState) -> Result<Action, handler::Error> {
    let token = r
        .headers
        .get_first("x-hype-auth-token")
        .ok_or(handler::Error::Status(status::UNAUTHORIZED.into()))?;

    if *token != state.token {
        return Err(handler::Error::Status((401, "Unauthorized").into()));
    }

    // Don't return a response as yet, simply pass the request on to the next handler.
    Ok(Action::Next)
}

#[derive(Debug, Deserialize, Clone)]
struct RouteConfig {
    #[serde(default)]
    pub id: RouteId,
    pub location: String,
    pub host_header: Option<String>,
    pub backends: Vec<BackendId>,
}

#[derive(Debug, Clone, Default)]
struct AppState {
    backends: Arc<RwLock<HashMap<lbconfig::BackendId, lbconfig::Backend>>>,
    routes: Arc<RwLock<HashMap<String, RouteConfig>>>,
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
        .get(id.as_ref())
        .ok_or(handler::Error::Status(status::NOT_FOUND.into()))?;

    Ok(format!("{:#?}", backend))
}

async fn add_route(r: Request, state: AppState) -> Result<String, handler::Error> {
    let route: RouteConfig = handlers::service::json(&r.body.content().await)?;

    {
        let lock = state.backends.read().await;
        if route.backends.iter().any(|b| !lock.contains_key(b)) {
            return Err(handler::Error::Status(status::NOT_FOUND.into()));
        }
    }

    let id = route.id.clone();
    state.routes.write().await.insert(id.clone().into(), route);
    Ok(format!("Got backend: {:?}", id))
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let args: Args = argh::from_env();

    let mut server = Server::new(&args.host, args.port);
    info!("Starting hype admin server on {}:{}", args.host, args.port);

    let middleware = Stack::new()
        .push(handlers::log())
        .push(handlers::service(auth).with_state(&AuthState {
            token: "foo".to_string(),
        }));

    let state = AppState {
        backends: Arc::new(RwLock::new(HashMap::new())),
        routes: Arc::new(RwLock::new(HashMap::new())),
    };

    server.route_method(
        Method::POST,
        "/backends",
        middleware
            .clone()
            .push(handlers::service(add_backend).with_state(&state)),
    );

    server.route_method(
        Method::GET,
        "/backends/:id",
        middleware
            .clone()
            .push(handlers::service(get_backend).with_state(&state)),
    );

    server.route_method(
        Method::POST,
        "/routes",
        middleware
            .clone()
            .push(handlers::service(add_route).with_state(&state)),
    );

    server.route_default(handlers::NotFoundHandler());
    server.start().await.unwrap();
}
