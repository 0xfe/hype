use std::{collections::HashMap, fmt, sync::Arc, time::Duration};

use rand::{thread_rng, Rng};
use tokio::{net::TcpStream, sync::RwLock};

use crate::client::ConnectedClient;

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
    conns: Arc<RwLock<HashMap<ConnId, Conn>>>,
}

impl ConnTracker {
    pub fn new() -> Self {
        Self {
            conns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn push_stream(&mut self, stream: TcpStream) -> Conn {
        let conn = Conn::new(stream);
        let id = conn.id.clone();
        self.conns.write().await.insert(id.clone(), conn.clone());
        conn
    }

    pub async fn stream(&self, id: &ConnId) -> Result<Arc<RwLock<TcpStream>>, String> {
        Ok(self
            .conns
            .read()
            .await
            .get(id)
            .ok_or(format!("could not get conn {}", id.0))?
            .stream())
    }
}

#[derive(Debug, Clone)]
pub struct ConnState {
    pub keepalive_timeout: Option<Duration>,
    pub keepalive_max: Option<usize>,
}

#[derive(Clone)]
pub struct Conn {
    id: ConnId,
    stream: Arc<RwLock<TcpStream>>,
    backend_client: Arc<RwLock<Option<ConnectedClient>>>, // for Lb
    pub state: Arc<std::sync::RwLock<ConnState>>,
}

impl fmt::Debug for Conn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Conn: {}\n, State: {:?}", self.id, self.state.read())
    }
}

impl Conn {
    pub fn new(stream: TcpStream) -> Self {
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
            state: Arc::new(std::sync::RwLock::new(ConnState {
                keepalive_timeout: None,
                keepalive_max: None,
            })),
        }
    }

    pub fn id(&self) -> &ConnId {
        &self.id
    }

    pub fn stream(&self) -> Arc<RwLock<TcpStream>> {
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
}
