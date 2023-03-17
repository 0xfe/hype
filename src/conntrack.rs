use std::{collections::HashMap, sync::Arc};

use rand::{thread_rng, Rng};
use tokio::{net::TcpStream, sync::RwLock};

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ConnId(String);

pub struct ConnTracker {
    conns: Arc<RwLock<HashMap<ConnId, Conn>>>,
}

impl ConnTracker {
    pub fn new() -> Self {
        Self {
            conns: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn push_stream(&mut self, stream: TcpStream) {
        let conn = Conn::new(stream);
        self.conns.write().await.insert(conn.id().clone(), conn);
    }

    pub async fn stream(&self, id: impl Into<String>) -> Result<Arc<RwLock<TcpStream>>, String> {
        let id = ConnId(id.into());

        Ok(self
            .conns
            .read()
            .await
            .get(&id)
            .ok_or(format!("could not get conn {}", id.0))?
            .stream()
            .await)
    }
}

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
