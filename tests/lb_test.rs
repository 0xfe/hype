use std::any::Any;

use async_trait::async_trait;

use hype::{
    client,
    handlers::lb::{self, Backend},
    request::Request,
    response::Response,
    status,
};

extern crate log;

#[derive(Debug)]
struct MockBackend {
    id: String,
    connect_attempts: usize,
    send_request_attempts: usize,
    requests: Vec<Request>,
    send_result: Option<Result<Response, client::ClientError>>,
}

impl MockBackend {
    pub fn new(id: impl Into<String>) -> Self {
        MockBackend {
            id: id.into(),
            connect_attempts: 0,
            send_request_attempts: 0,
            requests: vec![],
            send_result: Some(Ok(Response::new(status::from(status::OK)))),
        }
    }

    fn set_send_result(&mut self, result: Result<Response, client::ClientError>) {
        self.send_result = Some(result);
    }
}

#[async_trait]
impl Backend for MockBackend {
    async fn connect(&mut self) -> Result<(), client::ClientError> {
        self.connect_attempts += 1;
        Ok(())
    }

    async fn send_request(&mut self, req: &Request) -> Result<Response, client::ClientError> {
        println!("id: {}, request: {:?}", self.id, req);
        self.send_request_attempts += 1;
        self.requests.push(req.clone());

        // self.send_result.take().unwrap()
        Ok(Response::new(status::from(status::OK)))
    }
}

#[tokio::test]
async fn it_works() {
    let backend1 = Box::new(MockBackend::new("b1"));
    let backend2 = Box::new(MockBackend::new("b2"));
    let backend3 = Box::new(MockBackend::new("b3"));

    let mut lb = lb::Lb::new(lb::Policy::Random, vec![backend1, backend2, backend3]);

    println!("boo");

    lb.send_request(&Request::new("http://localhost:8000"))
        .await
        .unwrap();
    lb.send_request(&Request::new("http://localhost:8000"))
        .await
        .unwrap();
    lb.send_request(&Request::new("http://localhost:8000"))
        .await
        .unwrap();

    println!(
        "backend0.send_request_attempts: {}",
        lb.get_backend(0).unwrap().send_request_attempts
    );
    println!(
        "backend1.send_request_attempts: {}",
        lb.get_backend(1).unwrap().send_request_attempts
    );
    println!(
        "backend2.send_request_attempts: {}",
        lb.get_backend(2).unwrap().send_request_attempts
    );
}
