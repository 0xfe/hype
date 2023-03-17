use std::{collections::HashMap, fmt, sync::Arc};

use rand::{thread_rng, Rng};
use tokio::{net::TcpStream, sync::RwLock};

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

    pub async fn push_stream(&mut self, stream: TcpStream) -> ConnId {
        let conn = Conn::new(stream);
        let id = conn.id().clone();
        self.conns.write().await.insert(id.clone(), conn);
        id
    }

    pub async fn stream(&self, id: &ConnId) -> Result<Arc<RwLock<TcpStream>>, String> {
        Ok(self
            .conns
            .read()
            .await
            .get(id)
            .ok_or(format!("could not get conn {}", id.0))?
            .stream()
            .await)
    }
}

#[derive(Debug)]
struct Conn {
    id: ConnId,
    stream: Arc<RwLock<TcpStream>>,
}

impl Conn {
    fn new(stream: TcpStream) -> Self {
        Self {
            id: ConnId(
                thread_rng()
                    .sample_iter(&rand::distributions::Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect(),
            ),
            stream: Arc::new(RwLock::new(stream)),
        }
    }

    fn id(&self) -> &ConnId {
        &self.id
    }

    async fn stream(&self) -> Arc<RwLock<TcpStream>> {
        Arc::clone(&self.stream)
    }
}
