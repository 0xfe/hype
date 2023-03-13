use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use hype::{
    client,
    lb::{
        backend::Backend,
        http,
        picker::{Picker, RRPicker, RandomPicker, WeightedRRPicker},
    },
    request::Request,
    response::Response,
    status,
};

extern crate log;

#[derive(Debug, Clone)]
struct MockBackendStats {
    connect_attempts: usize,
    send_request_attempts: usize,
    requests: Vec<Request>,
}

#[derive(Debug)]
struct MockBackend {
    id: String,
    stats: Arc<Mutex<MockBackendStats>>,
}

impl MockBackend {
    pub fn new(id: impl Into<String>) -> Self {
        MockBackend {
            id: id.into(),
            stats: Arc::new(Mutex::new(MockBackendStats {
                connect_attempts: 0,
                send_request_attempts: 0,
                requests: vec![],
            })),
            // send_result: Some(Ok(Response::new(status::from(status::OK)))),
        }
    }
}

#[async_trait]
impl Backend for MockBackend {
    async fn connect(&self) -> Result<(), client::ClientError> {
        self.stats.lock().unwrap().connect_attempts += 1;
        Ok(())
    }

    async fn send_request(&self, req: &Request) -> Result<Response, client::ClientError> {
        println!("id: {}, request: {:?}", self.id, req);
        let mut stats = self.stats.lock().unwrap();
        stats.send_request_attempts += 1;
        stats.requests.push(req.clone());

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

    let lb = http::Http::new(backends, RandomPicker::new());

    for _ in 0..20 {
        lb.send_request(&Request::new("http://localhost:8000"))
            .await
            .unwrap();
    }

    let results = futures::future::join_all((0..3).map(|i| get_stats(&lb, i))).await;
    let total_requests: usize = results.iter().map(|r| r.send_request_attempts).sum();

    assert_eq!(total_requests, 20)
}

async fn get_stats<P: Picker<MockBackend>>(
    lb: &http::Http<MockBackend, P>,
    i: usize,
) -> MockBackendStats {
    // return (*lb.get_backend(i).unwrap().stats.lock().unwrap()).clone();
    return lb.get_backends().read().await[i]
        .stats
        .lock()
        .unwrap()
        .clone();
}

#[tokio::test]
async fn rr_policy() {
    let backends = vec![
        MockBackend::new("b1"),
        MockBackend::new("b2"),
        MockBackend::new("b3"),
        MockBackend::new("b4"),
    ];

    let lb = http::Http::new(backends, RRPicker::new());

    for _ in 0..20 {
        lb.send_request(&Request::new("http://localhost:8000"))
            .await
            .unwrap();
    }

    let results = futures::future::join_all((0..4).map(|i| get_stats(&lb, i))).await;
    let total_requests: usize = results.iter().map(|r| r.send_request_attempts).sum();

    assert_eq!(total_requests, 20);

    results
        .iter()
        .for_each(|r| assert_eq!(r.send_request_attempts, 5));
}

#[tokio::test]
async fn weighted_rr_policy() {
    let backends = vec![
        MockBackend::new("b1"),
        MockBackend::new("b2"),
        MockBackend::new("b3"),
        MockBackend::new("b4"),
    ];

    let lb = http::Http::new(backends, WeightedRRPicker::new(vec![3, 2, 1, 4]));

    for _ in 0..20 {
        lb.send_request(&Request::new("http://localhost:8000"))
            .await
            .unwrap();
    }

    let results = futures::future::join_all((0..4).map(|i| get_stats(&lb, i))).await;
    let total_requests: usize = results.iter().map(|r| r.send_request_attempts).sum();

    assert_eq!(total_requests, 20);

    assert_eq!(results[0].send_request_attempts, 6);
    assert_eq!(results[1].send_request_attempts, 4);
    assert_eq!(results[2].send_request_attempts, 2);
    assert_eq!(results[3].send_request_attempts, 8);
}
