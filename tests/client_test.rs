use hype::{client::Client, request::Request};

#[tokio::test]
async fn it_works() {
    let r = r##"GET / HTTP/1.1
Accept-Encoding: identity
Host: google.com"##;

    let mut client = Client::new("localhost:8080");
    let mut client = client.connect().await.unwrap();

    let req = Request::from(r, "http://localhost:8080").unwrap();
    let result = client.send_request(&req).await.unwrap();

    println!("result: {:?}", result);
}
