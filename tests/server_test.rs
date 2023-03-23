use std::time::Duration;

use async_trait::async_trait;
use hype::{
    client::Client,
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    server::Server,
    status,
};
use tokio::{io::AsyncWriteExt, time::sleep};

const HOST: &str = "127.0.0.1";

struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(
        &self,
        _r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_body("OK".into());
        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

#[tokio::test]
async fn it_works() {
    let port = 7855;
    let mut server = Server::new(HOST, port);
    server.route_default(Box::new(MyHandler {}));
    let ready = server.start_notifier();
    let (shutdown_command, shutdown_notifier) = server.shutdown();

    tokio::spawn(async move { server.start().await.unwrap() });

    ready.notified().await;

    let address = format!("{}:{}", HOST, port);

    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();

    let mut request = Request::new(format!("http://{}", address));
    request.set_method(hype::request::Method::GET);
    request.set_path("/");

    let response = client.send_request(&request).await.unwrap();
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");

    shutdown_command.send(true).await.unwrap();
    shutdown_notifier.notified().await;
    sleep(Duration::from_millis(1000)).await;
}

#[tokio::test]
async fn keep_alive() {
    let port = 7856;
    let mut server = Server::new(HOST, port);
    server.route_default(Box::new(MyHandler {}));
    let ready = server.start_notifier();
    let (shutdown_command, shutdown_notifier) = server.shutdown();

    tokio::spawn(async move { server.start().await.unwrap() });

    ready.notified().await;

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

    // SHUTDOWN SERVER
    shutdown_command.send(true).await.unwrap();
    shutdown_notifier.notified().await;

    let response = client.send_request(&request).await;
    assert_eq!(response.is_err(), true);
    assert_eq!(client.is_closed().await, true);
    // EXPECT ERROR
}
