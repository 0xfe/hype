use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use futures::StreamExt;
use hype::{
    client::{self, Client},
    handler::{self, AsyncWriteStream, Handler},
    handlers,
    lb::{
        backend::{Backend, HttpBackend},
        http::{self, Http},
        picker::{Picker, RRPicker, RandomPicker, WeightedRRPicker},
    },
    request::{Method, Request},
    response::Response,
    server::Server,
    status,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{mpsc, Notify},
};

#[macro_use]
extern crate log;

#[derive(Debug, Clone)]
struct MockBackendStats {
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
                send_request_attempts: 0,
                requests: vec![],
            })),
            // send_result: Some(Ok(Response::new(status::from(status::OK)))),
        }
    }
}

#[async_trait]
impl Backend for MockBackend {
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
        lb.send_request(&Request::new(hype::request::Method::GET, "/"))
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
        lb.send_request(&Request::new(hype::request::Method::GET, "/"))
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
        lb.send_request(&Request::new(hype::request::Method::GET, "/"))
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

async fn start_server(port: u16, text: String) -> (Arc<mpsc::Sender<bool>>, Arc<Notify>) {
    let handler = handlers::status::Status::new(status::from(status::OK), text);
    let mut server = Server::new("localhost", port);
    server.route_default(Box::new(handler));
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

async fn start_lb_backends(
    num_servers: usize,
    start_port: u16,
) -> (
    handlers::lb::Lb<RRPicker>,
    Vec<(Arc<mpsc::Sender<bool>>, Arc<Notify>)>,
) {
    let mut shutdowns = vec![];
    let mut backends = vec![];

    for i in 0..num_servers {
        let port = start_port + i as u16;
        debug!("Starting LB backend server on port: {}", port);
        let shutdown = start_server(port, format!("server{}", port)).await;
        shutdowns.push(shutdown);
        backends.push(HttpBackend::new(format!("localhost:{}", port)));
    }

    let balancer = Http::new(backends, RRPicker::new());
    let lb = hype::handlers::lb::Lb::new(balancer);

    (lb, shutdowns)
}

// Test the load balancer with real client and server tasks
#[tokio::test]
async fn lb_with_client_and_server() {
    hype::logger::init();
    let mut shutdowns = vec![];
    let (lb1, shutdowns1) = start_lb_backends(3, 10010).await;
    let (lb2, shutdowns2) = start_lb_backends(3, 10020).await;

    shutdowns.extend(shutdowns1);
    shutdowns.extend(shutdowns2);

    let mut lb_server = Server::new("localhost", 10099);
    lb_server.route("/lb1".to_string(), Box::new(lb1)).await;
    lb_server.route("/lb2".to_string(), Box::new(lb2)).await;

    let lb_ready = lb_server.start_notifier();
    let lb_shutdown = lb_server.shutdown();
    tokio::spawn(async move { lb_server.start().await.unwrap() });
    lb_ready.notified().await;

    let mut client = Client::new("localhost:10099");
    let mut client = client.connect().await.unwrap();

    // Hit the first backend in the set
    let response = client
        .send_request(&Request::new(Method::GET, "/lb1"))
        .await
        .unwrap();
    assert_eq!(response.body.content().await, "server10010".as_bytes());

    // Connection still open, stay on the first backend
    let response = client
        .send_request(&Request::new(Method::GET, "/lb1"))
        .await
        .unwrap();
    assert_eq!(response.body.content().await, "server10010".as_bytes());

    // Force close and reopen connection
    _ = client.close().await;
    let mut client = Client::new("localhost:10099");
    let mut client = client.connect().await.unwrap();

    // This should hit the second backend in the set
    let response = client
        .send_request(&Request::new(Method::GET, "/lb1"))
        .await
        .unwrap();
    assert_eq!(response.body.content().await, "server10012".as_bytes());

    // Force close and reopen connection
    _ = client.close().await;
    let mut client = Client::new("localhost:10099");
    let mut client = client.connect().await.unwrap();

    // This should hit the first backend in the second set
    let response = client
        .send_request(&Request::new(Method::GET, "/lb2"))
        .await
        .unwrap();
    assert_eq!(response.body.content().await, "server10020".as_bytes());

    for shutdown in shutdowns {
        shutdown_server(shutdown).await;
    }
    shutdown_server(lb_shutdown).await;
}

struct EchoHandler {}

#[async_trait]
impl Handler for EchoHandler {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        info!("EchoHandler Request: {:?}", r);

        let mut response = Response::new(status::from(status::OK));
        response.headers = r.headers.clone();

        w.write_all(
            format!(
                "{}\r\n{}\r\n\r\n",
                response.serialize_status(),
                response.headers.serialize()
            )
            .as_bytes(),
        )
        .await
        .unwrap();

        let body = &r.body;
        let mut stream = body.raw_stream();
        while let Some(chunk) = stream.next().await {
            w.write_all(chunk.as_slice()).await.unwrap();
        }

        Ok(handler::Ok::Next)
    }
}

async fn start_echo_server(port: u16) -> (Arc<mpsc::Sender<bool>>, Arc<Notify>) {
    let mut server = Server::new("localhost", port);
    server.route_default(Box::new(EchoHandler {}));
    let ready = server.start_notifier();
    let shutdown = server.shutdown();

    tokio::spawn(async move { server.start().await.unwrap() });
    ready.notified().await;
    shutdown
}

async fn start_streaming_backends(
    num_servers: usize,
    start_port: u16,
) -> (
    handlers::lb::Lb<RRPicker>,
    Vec<(Arc<mpsc::Sender<bool>>, Arc<Notify>)>,
) {
    let mut shutdowns = vec![];
    let mut backends = vec![];

    for i in 0..num_servers {
        let port = start_port + i as u16;
        debug!("Starting LB backend server on port: {}", port);
        let shutdown = start_echo_server(port).await;
        shutdowns.push(shutdown);
        backends.push(HttpBackend::new(format!("localhost:{}", port)));
    }

    let balancer = Http::new(backends, RRPicker::new());
    let lb = hype::handlers::lb::Lb::new(balancer);

    (lb, shutdowns)
}

// Test streaming requests and responses through the load balancer
#[tokio::test]
async fn streaming_lb() {
    hype::logger::init();
    let (lb, shutdowns) = start_streaming_backends(3, 10100).await;

    let mut lb_server = Server::new("localhost", 10199);
    lb_server.route("/lb".to_string(), Box::new(lb)).await;

    let lb_ready = lb_server.start_notifier();
    let lb_shutdown = lb_server.shutdown();
    tokio::spawn(async move { lb_server.start().await.unwrap() });
    lb_ready.notified().await;

    let mut client = Client::new("localhost:10199");
    let mut client = client.connect().await.unwrap();

    let request = &mut Request::new(Method::GET, "/lb");
    request.headers.set("content-length", "18");
    request.body.set_content_length(18);

    // Hit the first backend in the set
    let response = client.send_request(request).await.unwrap();

    let mut stream = response.body.content_stream();
    println!("Response: {:?}", response);

    request.body.append("foobar".as_bytes()).unwrap();
    assert_eq!(
        "foobar",
        String::from_utf8_lossy(stream.next().await.unwrap().as_slice())
    );

    assert_eq!(request.body.complete(), false);

    request.body.append("foobar".as_bytes()).unwrap();
    assert_eq!(
        "foobar",
        String::from_utf8_lossy(stream.next().await.unwrap().as_slice())
    );

    request.body.append("foobar".as_bytes()).unwrap();
    assert_eq!(
        "foobar",
        String::from_utf8_lossy(stream.next().await.unwrap().as_slice())
    );

    assert_eq!(request.body.complete(), true);

    for shutdown in shutdowns {
        shutdown_server(shutdown).await;
    }
    shutdown_server(lb_shutdown).await;
}

// Test streaming chunked requests throught the load balancer
#[tokio::test]
async fn streaming_lb_chunked() {
    hype::logger::init();
    let (lb, shutdowns) = start_streaming_backends(3, 10200).await;

    let mut lb_server = Server::new("localhost", 10299);
    lb_server.route("/lb".to_string(), Box::new(lb)).await;

    let lb_ready = lb_server.start_notifier();
    let lb_shutdown = lb_server.shutdown();
    tokio::spawn(async move { lb_server.start().await.unwrap() });
    lb_ready.notified().await;

    let mut client = Client::new("localhost:10299");
    let mut client = client.connect().await.unwrap();

    let request = &mut Request::new(Method::GET, "/lb");
    request.set_chunked();

    // Hit the first backend in the set
    let response = client.send_request(request).await.unwrap();
    println!("Response: {:?}", response);

    let mut stream = response.body.chunk_stream();

    request.body.push_chunk("foobar".as_bytes().to_vec());
    assert_eq!(
        "foobar",
        String::from_utf8_lossy(stream.next().await.unwrap().as_slice())
    );

    request.body.push_chunk("foobar".as_bytes().to_vec());
    assert_eq!(
        "foobar",
        String::from_utf8_lossy(stream.next().await.unwrap().as_slice())
    );

    request.body.push_chunk("foobar".as_bytes().to_vec());
    assert_eq!(
        "foobar",
        String::from_utf8_lossy(stream.next().await.unwrap().as_slice())
    );

    request.body.end_chunked();

    assert_eq!(stream.next().await, None);

    for shutdown in shutdowns {
        shutdown_server(shutdown).await;
    }
    shutdown_server(lb_shutdown).await;
}
