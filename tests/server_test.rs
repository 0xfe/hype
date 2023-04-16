use std::{sync::Arc, time::Duration};

use async_trait::async_trait;
use hype::{
    client::Client,
    handler::{self, AsyncWriteStream, Handler},
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
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Action, handler::Error> {
        if r.headers.get_first("x-hype-test-force-error").is_some() {
            match r
                .headers
                .get_first("x-hype-test-force-error")
                .unwrap()
                .as_str()
            {
                "Next" => {
                    let mut response = Response::new(status::OK);
                    response.set_body("Next");
                    let buf = response.serialize();
                    w.write_all(buf.as_bytes()).await.unwrap();
                    return Ok(handler::Action::Next);
                }
                "Done" => {
                    let mut response = Response::new(status::OK);
                    response.set_body("Done");
                    let buf = response.serialize();
                    w.write_all(buf.as_bytes()).await.unwrap();
                    return Ok(handler::Action::Done);
                }
                "Redirect" => {
                    return Ok(handler::Action::Redirect(
                        r.headers
                            .get_first("x-hype-test-redirect-location")
                            .unwrap()
                            .to_string(),
                    ));
                }
                "Failed" => {
                    return Err(handler::Error::Failed("Forced error".to_string()));
                }
                "CustomStatus" => {
                    return Err(handler::Error::Status(
                        (
                            r.headers
                                .get_first("x-hype-test-customstatus-code")
                                .unwrap()
                                .parse()
                                .unwrap(),
                            r.headers
                                .get_first("x-hype-test-customstatus-message")
                                .unwrap()
                                .to_string(),
                        )
                            .into(),
                    ));
                }
                _ => {}
            }
        }

        let mut response = Response::new(status::OK);
        response.set_body("OK");

        response.headers.set(
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

        response.headers.set(
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

        Ok(handler::Action::Done)
    }
}

async fn start_server(port: u16) -> (Arc<mpsc::Sender<bool>>, Arc<Notify>) {
    let mut server = Server::new(HOST, port);
    server.route_default(MyHandler {});
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
    let request = Request::default();

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn keep_alive() {
    let port = 7856;
    let shutdown = start_server(port).await;
    let address = format!("{}:{}", HOST, port);
    let request = Request::default();

    // Create new connection
    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());
    assert_eq!(client.is_closed().await, false);

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());
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

    let mut request = Request::default();
    request.headers.set("Connection", "Keep-Alive");
    request.headers.set("Keep-Alive", "timeout=10, max=5");

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());
    assert_eq!(
        response
            .headers
            .get_first("x-hype-test-keepalive-timeout")
            .unwrap(),
        "10"
    );
    assert_eq!(
        response
            .headers
            .get_first("x-hype-test-keepalive-max")
            .unwrap(),
        "5"
    );

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn keep_alive_timeout() {
    let port = 8858;
    let address = format!("{}:{}", HOST, port);
    let shutdown = start_server(port).await;

    let mut request = Request::default();
    request.headers.set("Connection", "Keep-Alive");
    request.headers.set("Keep-Alive", "timeout=1");

    // Create new connection
    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());
    assert_eq!(client.is_closed().await, false);

    tokio::time::sleep(Duration::from_secs(2)).await;

    let response = client.send_request(&request).await;
    assert_eq!(response.is_err(), true);
    assert_eq!(client.is_closed().await, true);

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn keep_alive_max() {
    hype::logger::init();

    let port = 8859;
    let address = format!("{}:{}", HOST, port);
    let shutdown = start_server(port).await;

    let mut request = Request::default();
    request.headers.set("Connection", "Keep-Alive");
    request.headers.set("Keep-Alive", "max=2");

    // Create new connection
    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());
    assert_eq!(client.is_closed().await, false);

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body.content().await, "OK".as_bytes());
    assert_eq!(client.is_closed().await, false);

    let response = client.send_request(&request).await;
    assert_eq!(response.is_err(), true);
    assert_eq!(client.is_closed().await, true);

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn connection_close() {
    hype::logger::init();
    let port = 8860;
    let address = format!("{}:{}", HOST, port);
    let shutdown = start_server(port).await;

    let mut request = Request::default();
    request.headers.set("Connection", "close");

    // Create new connection
    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.content().await, "OK");
    assert_eq!(client.is_closed().await, false);

    let response = client.send_request(&request).await;
    assert_eq!(response.is_err(), true);
    assert_eq!(client.is_closed().await, true);

    shutdown_server(shutdown).await;
}

#[tokio::test]
async fn default_error() {
    let port = 8861;
    let address = format!("{}:{}", HOST, port);
    let shutdown = start_server(port).await;

    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();

    let mut request = Request::default();
    request
        .headers
        .set("x-hype-test-force-error", "CustomStatus");
    request.headers.set("x-hype-test-customstatus-code", "600");
    request
        .headers
        .set("x-hype-test-customstatus-message", "CustomStatus");

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 600);
    assert_eq!(response.content().await, "600 CustomStatus");

    let mut request = Request::default();
    request.headers.set("x-hype-test-force-error", "Failed");
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 500);
    assert_eq!(
        response.content().await,
        "500 INTERNAL SERVER ERROR - Forced error"
    );

    let mut request = Request::default();
    request.headers.set("x-hype-test-force-error", "Redirect");
    request
        .headers
        .set("x-hype-test-redirect-location", "http://foo");
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 301);
    assert_eq!(
        response.headers.get_first("location").unwrap(),
        "http://foo"
    );

    let mut request = Request::default();
    request.headers.set("x-hype-test-force-error", "Done");
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.content().await, "Done");

    let mut request = Request::default();
    request.headers.set("x-hype-test-force-error", "Next");
    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.content().await, "Next");

    // Shutdown server
    shutdown_server(shutdown).await;
}
