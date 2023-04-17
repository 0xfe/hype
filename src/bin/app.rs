use std::sync::Arc;

use hype::{handlers, request::Method, server::Server};
use tokio::sync::Mutex;

#[derive(Debug, Clone, Default)]
struct AppState {
    counter: Arc<Mutex<u32>>,
}

#[tokio::main]
async fn main() {
    hype::logger::init();

    let app = AppState {
        counter: Arc::new(Mutex::new(0)),
    };

    let mut server = Server::new("127.0.0.1", 4000);
    server.route_method(
        Method::GET,
        "/counter/get",
        handlers::service(
            |_, s: AppState| async move { Ok(format!("{}\n", s.counter.lock().await)) },
        )
        .with_state(&app),
    );

    server.route_method(
        Method::POST,
        "/counter/inc",
        handlers::service(|_, s: AppState| async move {
            *s.counter.lock().await += 1;
            Ok("OK\n")
        })
        .with_state(&app),
    );

    server.start().await.unwrap();
}
