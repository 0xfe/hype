use std::{
    pin::Pin,
    task::{Context, Poll, Waker},
};

use futures::Stream;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk(pub String);

#[derive(Debug)]
struct ChunkState {
    // chunked body
    chunks: Vec<Chunk>,
    chunks_complete: bool,
    wakers: Vec<Waker>,
}

#[derive(Debug)]
pub struct Body {
    full_body: String,
    chunked: bool,
    chunk_state: std::sync::Arc<std::sync::RwLock<ChunkState>>,
}

impl Body {
    pub fn new() -> Self {
        return Self {
            full_body: String::new(),
            chunked: false,
            chunk_state: std::sync::Arc::new(std::sync::RwLock::new(ChunkState {
                chunks: vec![],
                chunks_complete: false,
                wakers: vec![],
            })),
        };
    }

    pub fn set_chunked(&mut self) {
        self.chunked = true;
    }

    pub fn push_chunk(&self, chunk: impl Into<String>) {
        if !self.chunked {
            panic!("push_chunk: not chunked")
        }

        let mut chunk_state = self.chunk_state.write().unwrap();
        chunk_state.chunks.push(Chunk(chunk.into()));
        chunk_state.wakers.iter().for_each(|w| w.clone().wake());
        chunk_state.wakers.clear();
    }

    pub fn end_chunked(&self) {
        if !self.chunked {
            panic!("end_chunked: not chunked")
        }

        let mut chunk_state = self.chunk_state.write().unwrap();
        if chunk_state.chunks_complete {
            panic!("chunks already complete")
        }

        chunk_state.chunks_complete = true;
        chunk_state.wakers.iter().for_each(|w| w.clone().wake());
        chunk_state.wakers.clear();
    }

    pub fn get_chunk(&self, i: usize, waker: Waker) -> Result<Chunk, bool> {
        let mut chunk_state = self.chunk_state.write().unwrap();
        if i >= chunk_state.chunks.len() {
            if !chunk_state.chunks_complete {
                chunk_state.wakers.push(waker);
            }
            return Err(chunk_state.chunks_complete);
        }

        Ok(chunk_state.chunks[i].clone())
    }

    pub fn append_body(&mut self, buf: impl AsRef<str>) {
        self.full_body.push_str(buf.as_ref())
    }

    pub fn body(&self) -> String {
        if !self.chunked {
            return self.full_body.clone();
        }

        let chunk_state = self.chunk_state.read().unwrap();

        if !chunk_state.chunks_complete {
            panic!("body(): incomplete chunks")
        }

        return chunk_state
            .chunks
            .iter()
            .map(|c| c.0.clone())
            .collect::<Vec<String>>()
            .join("")
            .to_string();
    }

    pub fn stream(&self) -> BodyStream {
        if !self.chunked {
            panic!("stream(): not chunked")
        }

        BodyStream {
            current_chunk: 0,
            body: self,
        }
    }
}

pub struct BodyStream<'a> {
    current_chunk: usize,
    body: &'a Body,
}

impl<'a> Stream for BodyStream<'a> {
    type Item = Chunk;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Chunk>> {
        let chunk = self.body.get_chunk(self.current_chunk, cx.waker().clone());

        match chunk {
            Ok(chunk) => {
                self.current_chunk += 1;
                Poll::Ready(Some(chunk))
            }
            Err(true) => Poll::Ready(None),
            Err(false) => Poll::Pending,
        }
    }
}
