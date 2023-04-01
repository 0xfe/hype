use std::{
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::Stream;

#[derive(Debug)]
pub struct Body {
    full_body: String,

    // chunked body
    chunked: std::sync::Arc<std::sync::RwLock<bool>>,
    chunks: Vec<String>,
    chunks_complete: bool,

    // stream
    current_chunk: usize,
    cx: Option<Waker>,
}

impl Body {
    pub fn new() -> Self {
        return Self {
            chunked: std::sync::Arc::new(std::sync::RwLock::new(false)),
            full_body: String::new(),
            chunks: vec![],
            chunks_complete: false,
            current_chunk: 0,
            cx: None,
        };
    }

    pub fn set_chunked(&mut self) {
        *self.chunked.write().unwrap() = true;
    }

    pub fn push_chunk(&mut self, chunk: impl Into<String>) {
        let chunked = self.chunked.write().unwrap();

        if !*chunked {
            panic!("push_chunk: not chunked")
        }

        self.chunks.push(chunk.into());

        if let Some(waker) = &self.cx {
            waker.clone().wake();
        }
    }

    pub fn end_chunked(&mut self) {
        let chunked = self.chunked.write().unwrap();

        if !*chunked {
            panic!("end_chunked: not chunked")
        }

        if self.chunks_complete {
            panic!("chunks already complete")
        }
        self.chunks_complete = true;
    }

    pub fn append_body(&mut self, buf: impl AsRef<str>) {
        self.full_body.push_str(buf.as_ref())
    }

    pub fn body(&self) -> String {
        let chunked = self.chunked.read().unwrap();
        if *chunked {
            if !self.chunks_complete {
                panic!("body(): incomplete chunks")
            }
            return self.chunks.join("").to_string();
        }
        return self.full_body.clone();
    }
}

impl Stream for Body {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<String>> {
        let chunked = *self.chunked.read().unwrap();

        if !chunked {
            // No more delays
            return Poll::Ready(None);
        }

        if self.current_chunk < self.chunks.len() {
            self.current_chunk += 1;
            if let Some(chunk) = self.chunks.get(self.current_chunk - 1) {
                return Poll::Ready(Some(chunk.clone()));
            } else {
                return Poll::Ready(None);
            }
        } else {
            if self.chunks_complete {
                return Poll::Ready(None);
            }
        }

        self.cx = Some(cx.waker().clone());

        return Poll::Pending;
    }
}
