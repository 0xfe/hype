use std::{collections::HashMap, sync::Arc};

use tokio::sync::RwLock;

use crate::{client::ClientError, request::Request, response::Response};

use super::{backend::Backend, picker::Picker};

pub struct Http<T: Backend, P: Picker<T>> {
    backends: Arc<RwLock<Vec<T>>>,
    picker: P,
    headers: HashMap<String, String>,
}

impl<T: Backend, P: Picker<T>> Http<T, P> {
    pub fn new(backends: Vec<T>, picker: P) -> Self {
        Self {
            backends: Arc::new(RwLock::new(backends)),
            picker,
            headers: HashMap::new(),
        }
    }

    pub fn rewrite_header(&mut self, k: impl Into<String>, v: impl Into<String>) {
        self.headers.insert(k.into(), v.into());
    }

    pub async fn send_request(&self, req: &Request) -> Result<Response, ClientError> {
        let backends = self.backends.read().await;
        let index = self
            .picker
            .pick_backend(&*backends)
            .map_err(|e| ClientError::InternalError(format!("could not pick backend: {}", e)))?;

        if index > backends.len() {
            return Err(ClientError::InternalError(format!(
                "picker returned invalid index: {}, num backends: {}",
                index,
                backends.len()
            )));
        }

        // Rewrite headers as needed
        let mut req = req.clone();
        self.headers.iter().for_each(|(k, v)| req.set_header(k, v));

        debug!("LB: sending request to backend {}: {:?}", index, req);
        backends[index].send_request(&req).await
    }

    pub fn get_backends(&self) -> Arc<RwLock<Vec<T>>> {
        Arc::clone(&self.backends)
    }
}
