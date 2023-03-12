use async_trait::async_trait;

use hype::{
    client,
    handlers::lb::{self, Backend, RRPicker, RandomPicker},
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
}

impl MockBackend {
    pub fn new(id: impl Into<String>) -> Self {
        MockBackend {
            id: id.into(),
            connect_attempts: 0,
            send_request_attempts: 0,
            requests: vec![],
            // send_result: Some(Ok(Response::new(status::from(status::OK)))),
        }
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
async fn random_policy() {
    let backends = vec![
        MockBackend::new("b1"),
        MockBackend::new("b2"),
        MockBackend::new("b3"),
    ];

    let mut lb = lb::Lb::new(backends, RandomPicker::new());

    for _ in 0..20 {
        lb.send_request(&Request::new("http://localhost:8000"))
            .await
            .unwrap();
    }

    let total_requests: usize = (0..3)
        .map(|i| lb.get_backend(i).unwrap().send_request_attempts)
        .sum();

    assert_eq!(total_requests, 20)
}

#[tokio::test]
async fn rr_policy() {
    let backends = vec![
        MockBackend::new("b1"),
        MockBackend::new("b2"),
        MockBackend::new("b3"),
        MockBackend::new("b4"),
    ];

    let mut lb = lb::Lb::new(backends, RRPicker::new());

    for _ in 0..20 {
        lb.send_request(&Request::new("http://localhost:8000"))
            .await
            .unwrap();
    }

    let total_requests: usize = (0..4)
        .map(|i| lb.get_backend(i).unwrap().send_request_attempts)
        .sum();

    assert_eq!(total_requests, 20);
    (0..4).for_each(|i| assert_eq!(lb.get_backend(i).unwrap().send_request_attempts, 5));
}
