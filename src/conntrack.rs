use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use futures::StreamExt;
use rand::{thread_rng, Rng};
use tokio::{
    select,
    sync::{mpsc, Mutex, Notify, RwLock},
};
use tokio_util::time::DelayQueue;

use crate::{client::ConnectedClient, handler::AsyncStream};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ConnId(pub String);

impl fmt::Display for ConnId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<ConnId> for String {
    fn from(val: ConnId) -> Self {
        val.0
    }
}

#[derive(Debug)]
pub struct ConnTracker {
    conns: Arc<std::sync::RwLock<HashMap<ConnId, Conn>>>,
    keepalive_tx: mpsc::Sender<(ConnId, Duration)>,
    keepalive_rx: Arc<Mutex<mpsc::Receiver<(ConnId, Duration)>>>,
    shutdown_notifier: Arc<Notify>,
}

impl ConnTracker {
    pub fn new() -> Self {
        let (keepalive_tx, keepalive_rx) = mpsc::channel(1);

        Self {
            conns: Arc::new(std::sync::RwLock::new(HashMap::new())),
            keepalive_tx,
            keepalive_rx: Arc::new(Mutex::new(keepalive_rx)),
            shutdown_notifier: Arc::new(Notify::new()),
        }
    }

    pub fn push_stream(&mut self, stream: Box<dyn AsyncStream>) -> Conn {
        let conn = Conn::new(stream);
        let id = conn.id.clone();
        self.conns.write().unwrap().insert(id.clone(), conn.clone());
        conn
    }

    pub async fn stream(&self, id: &ConnId) -> Result<Arc<RwLock<Box<dyn AsyncStream>>>, String> {
        Ok(self
            .conns
            .read()
            .unwrap()
            .get(id)
            .ok_or(format!("could not get conn {}", id.0))?
            .stream())
    }

    pub async fn set_keepalive_timeout(&self, id: ConnId, dur: Duration) {
        self.keepalive_tx.send((id, dur)).await.unwrap();
    }

    pub fn shutdown(&self) {
        self.shutdown_notifier.notify_one();
    }

    pub async fn process_keepalives(&self) {
        let mut keepalive_queue = DelayQueue::new();
        keepalive_queue.insert(ConnId("(0)".to_string()), Duration::from_secs(2000000));
        let conns = Arc::clone(&self.conns);
        let keepalive_rx = Arc::clone(&self.keepalive_rx);
        let shutdown_notifier = Arc::clone(&self.shutdown_notifier);

        info!("starting keepalive processor...");
        tokio::spawn(async move {
            let mut keepalive_rx = keepalive_rx.lock().await;

            loop {
                let conn_id = select! {
                    conn_id = keepalive_queue.next() => { conn_id },
                    Some((conn_id, dur)) = keepalive_rx.recv() => { keepalive_queue.insert(conn_id, dur); None }
                    _ = shutdown_notifier.notified() => { info!("shutting down connection tracker..."); break; }
                };

                if let Some(conn_id) = conn_id {
                    let mut conns = conns.write().unwrap();
                    conns.get(conn_id.get_ref()).unwrap().timeout_notify();
                    conns.remove(conn_id.get_ref());
                }
            }
        });
    }
}

#[derive(Debug, Clone)]
pub struct ConnState {
    pub keepalive_timeout: Option<Duration>,
    pub keepalive_max: Option<usize>,
    pub request_count: usize,
}

#[derive(Clone)]
pub struct Conn {
    id: ConnId,
    stream: Arc<RwLock<Box<dyn AsyncStream>>>,
    backend_client: Arc<RwLock<Option<ConnectedClient>>>, // for Lb
    timeout_notifier: Arc<Notify>,
    pub state: Arc<std::sync::RwLock<ConnState>>,
}

impl fmt::Debug for Conn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Conn: {}\n, State: {:?}", self.id, self.state.read())
    }
}

impl Conn {
    pub fn new(stream: Box<dyn AsyncStream>) -> Self {
        Self {
            id: ConnId(
                thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect(),
            ),
            stream: Arc::new(RwLock::new(stream)),
            backend_client: Arc::new(RwLock::new(None)),
            timeout_notifier: Arc::new(Notify::new()),
            state: Arc::new(std::sync::RwLock::new(ConnState {
                keepalive_timeout: None,
                keepalive_max: None,
                request_count: 0,
            })),
        }
    }

    pub fn id(&self) -> &ConnId {
        &self.id
    }

    pub fn stream(&self) -> Arc<RwLock<Box<dyn AsyncStream>>> {
        Arc::clone(&self.stream)
    }

    pub fn backend_client(&self) -> Arc<RwLock<Option<ConnectedClient>>> {
        Arc::clone(&self.backend_client)
    }

    pub async fn set_backend_client(&self, client: ConnectedClient) {
        *self.backend_client.write().await = Some(client)
    }

    pub fn set_keepalive_timeout(&mut self, dur: Duration) {
        self.state.write().unwrap().keepalive_timeout = Some(dur);
    }

    pub fn set_keepalive_max(&mut self, max: usize) {
        self.state.write().unwrap().keepalive_max = Some(max);
    }

    pub fn inc_request_count(&mut self) -> bool {
        let mut state = self.state.write().unwrap();
        state.request_count += 1;

        if let Some(max) = state.keepalive_max {
            return state.request_count > max;
        }

        false
    }

    pub fn timeout_notifier(&self) -> Arc<Notify> {
        Arc::clone(&self.timeout_notifier)
    }

    pub fn timeout_notify(&self) {
        self.timeout_notifier.notify_one()
    }
}
