use std::sync::Arc;

use tokio::sync::RwLock;

use crate::{client::ClientError, request::Request, response::Response};

use super::{backend::Backend, picker::Picker};

pub struct Http<T: Backend, P: Picker<T>> {
    backends: Arc<RwLock<Vec<T>>>,
    picker: P,
}

impl<T: Backend, P: Picker<T>> Http<T, P> {
    pub fn new(backends: Vec<T>, picker: P) -> Self {
        Self {
            backends: Arc::new(RwLock::new(backends)),
            picker,
        }
    }

    pub async fn send_request(&self, req: &Request) -> Result<Response, ClientError> {
        let backends = self.backends.read().await;
        let index = self
            .picker
            .pick_backend(&*backends)
            .map_err(|e| ClientError::OtherError(format!("could not pick backend: {}", e)))?;

        if index > backends.len() {
            return Err(ClientError::OtherError(format!(
                "picker returned invalid index: {}, num backends: {}",
                index,
                backends.len()
            )));
        }

        debug!("LB: sending request to backend {}: {:?}", index, req);
        backends[index].send_request(req).await
    }

    pub fn get_backends(&self) -> Arc<RwLock<Vec<T>>> {
        Arc::clone(&self.backends)
    }
}

#[cfg(test)]
mod tests {
    use crate::lb::{backend::HttpBackend, picker::RRPicker};

    use super::*;

    #[tokio::test]
    async fn it_works() {
        // let backend = Backend::new("142.251.33.174:80"); // google.com
        let backend = HttpBackend::new("127.0.0.1:8080");
        let lb = Http::new(vec![backend], RRPicker::new());

        let r = r##"GET / HTTP/1.1
Accept-Encoding: identity
Host: google.com"##;

        let req = Request::from(r, "http://google.com").unwrap();
        let response = lb.send_request(&req).await.unwrap();

        assert_eq!(response.status.code, 200);
        assert_eq!(response.status.text, "OK");
        assert_eq!(response.headers.get("connection").unwrap(), "keep-alive");
    }
}
