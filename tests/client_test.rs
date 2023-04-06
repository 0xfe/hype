use hype::{client::Client, request::Request};

#[tokio::test]
async fn it_works() {
    hype::logger::init();
    let r = r##"GET / HTTP/1.1
Accept-Encoding: identity
Host: google.com"##;

    let mut client = Client::new("google.com:80");
    let mut client = client.connect().await.unwrap();

    let req = Request::from(r).unwrap();
    let result = client.send_request(&req).await.unwrap();

    println!("result: {:?}", result);
}
