use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use hype::{
    client::Client,
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    server::Server,
    status,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc, Notify},
};

const HOST: &str = "127.0.0.1";

struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_body("OK".into());

        response.set_header(
            "x-hype-test-keepalive-timeout",
            r.conn()
                .unwrap()
                .state
                .read()
                .unwrap()
                .keepalive_timeout
                .unwrap_or(Duration::from_secs(0))
                .as_secs()
                .to_string(),
        );

        response.set_header(
            "x-hype-test-keepalive-max",
            r.conn()
                .unwrap()
                .state
                .read()
                .unwrap()
                .keepalive_max
                .unwrap_or(0)
                .to_string(),
        );

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

async fn start_server(port: u16) -> (Arc<mpsc::Sender<bool>>, Arc<Notify>) {
    let mut server = Server::new(HOST, port);
    server.route_default(Box::new(MyHandler {}));
    let ready = server.start_notifier();
    let shutdown = server.shutdown();

    tokio::spawn(async move { server.start().await.unwrap() });
    ready.notified().await;
    shutdown
}

async fn shutdown_server(shutdown: (Arc<mpsc::Sender<bool>>, Arc<Notify>)) {
    shutdown.0.send(true).await.unwrap();
    shutdown.1.notified().await;
}

#[tokio::test]
async fn it_works() {
    let port = 7855;
    let shutdown = start_server(port).await;

    let address = format!("{}:{}", HOST, port);

    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();

    let mut request = Request::new(format!("http://{}", address));
    request.set_method(hype::request::Method::GET);
    request.set_path("/");

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn keep_alive() {
    let port = 7856;
    let shutdown = start_server(port).await;

    let address = format!("{}:{}", HOST, port);

    let mut request = Request::new(format!("http://{}", address));
    request.set_method(hype::request::Method::GET);
    request.set_path("/");

    // Create new connection
    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");
    assert_eq!(client.is_closed().await, false);

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");
    assert_eq!(client.is_closed().await, false);

    shutdown_server(shutdown).await;

    let response = client.send_request(&request).await;
    assert_eq!(response.is_err(), true);
    assert_eq!(client.is_closed().await, true);
    // EXPECT ERROR
}

#[tokio::test]
async fn process_headers() {
    let port = 7857;
    let shutdown = start_server(port).await;

    let address = format!("{}:{}", HOST, port);

    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();

    let mut request = Request::new(format!("http://{}", address));
    request.set_method(hype::request::Method::GET);
    request.set_path("/");
    request.set_header("Connection", "Keep-Alive");
    request.set_header("Keep-Alive", "timeout=10, max=5");

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");
    assert_eq!(
        response
            .headers
            .get("x-hype-test-keepalive-timeout")
            .unwrap(),
        "10"
    );
    assert_eq!(
        response.headers.get("x-hype-test-keepalive-max").unwrap(),
        "5"
    );

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn keep_alive_timeout() {
    let port = 8858;
    let address = format!("{}:{}", HOST, port);
    start_server(port).await;

    let mut request = Request::new(format!("http://{}", address));
    request.set_method(hype::request::Method::GET);
    request.set_path("/");
    request.set_header("Connection", "Keep-Alive");
    request.set_header("Keep-Alive", "timeout=1");

    // Create new connection
    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");
    assert_eq!(client.is_closed().await, false);

    tokio::time::sleep(Duration::from_secs(2)).await;

    let response = client.send_request(&request).await;
    assert_eq!(response.is_err(), true);
    assert_eq!(client.is_closed().await, true);
    // EXPECT ERROR
}
