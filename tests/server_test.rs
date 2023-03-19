use async_trait::async_trait;
use hype::{
    client::Client,
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    server::Server,
    status,
};
use tokio::io::AsyncWriteExt;

const HOST: &str = "127.0.0.1";
const PORT: u16 = 7855;

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
    let mut server = Server::new(HOST, PORT);
    server.route_default(Box::new(MyHandler {}));
    tokio::spawn(async move { server.start().await.unwrap() });

    // This could be flaky
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    let address = format!("{}:{}", HOST, PORT);

    let mut client = Client::new(address.clone());
    let mut client = client.connect().await.unwrap();

    let mut request = Request::new(format!("http://{}", address));
    request.set_method(hype::request::Method::GET);
    request.set_path("/");

    let response = client.send_request(&request).await.unwrap();
    println!("RESPINSE: {:?}", response);
    assert_eq!(response.status.code, 200);
    assert_eq!(response.body, "OK");
}
