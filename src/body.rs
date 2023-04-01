use std::{
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll, Waker},
};

use futures::Stream;

/// We have two body types, based on their encoding: Chunked and Full.
#[derive(Debug)]
enum Content {
    Chunked(Arc<RwLock<ChunkState>>),
    Full(Arc<RwLock<String>>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk(pub String);

#[derive(Debug)]
struct ChunkState {
    // chunked body
    chunks: Vec<Chunk>,

    // no more chunks
    complete: bool,

    // wakers for stream futures
    wakers: Vec<Waker>,
}

impl ChunkState {
    fn new() -> Self {
        ChunkState {
            chunks: vec![],
            complete: false,
            wakers: vec![],
        }
    }
}

#[derive(Debug)]
pub struct Body {
    content: Content,
}

impl Body {
    pub fn new() -> Self {
        Self {
            content: Content::Full(Arc::new(RwLock::new(String::new()))),
        }
    }

    pub fn set_chunked(&mut self) {
        self.content = Content::Chunked(Arc::new(RwLock::new(ChunkState::new())));
    }

    pub fn chunked(&self) -> bool {
        if let Content::Chunked(_) = self.content {
            return true;
        }

        return false;
    }

    pub fn push_chunk(&self, chunk: impl Into<String>) {
        match &self.content {
            Content::Full(_) => panic!("not chunked"),
            Content::Chunked(state) => {
                // Notify async stream wakers that there's a new chunk
                let mut wakers = vec![];
                {
                    let mut chunk_state = state.write().unwrap();
                    chunk_state.chunks.push(Chunk(chunk.into()));
                    std::mem::swap(&mut wakers, &mut chunk_state.wakers);
                }
                wakers.iter().for_each(|w| w.wake_by_ref());
            }
        }
    }

    pub fn end_chunked(&self) {
        match &self.content {
            Content::Full(_) => panic!("not chunked"),
            Content::Chunked(state) => {
                let mut wakers = vec![];
                {
                    let mut chunk_state = state.write().unwrap();
                    if chunk_state.complete {
                        panic!("chunks already complete")
                    }

                    chunk_state.complete = true;
                    std::mem::swap(&mut wakers, &mut chunk_state.wakers);
                }
                wakers.iter().for_each(|w| w.wake_by_ref());
            }
        }
    }

    pub fn get_chunk(&self, i: usize, waker: Waker) -> Result<Chunk, bool> {
        match &self.content {
            Content::Full(_) => panic!("not chunked"),
            Content::Chunked(state) => {
                let mut chunk_state = state.write().unwrap();
                if i >= chunk_state.chunks.len() {
                    if !chunk_state.complete {
                        chunk_state.wakers.push(waker);
                    }
                    return Err(chunk_state.complete);
                }

                Ok(chunk_state.chunks[i].clone())
            }
        }
    }

    pub fn append_body(&mut self, buf: impl AsRef<str>) {
        match &self.content {
            Content::Full(body) => body.write().unwrap().push_str(buf.as_ref()),
            Content::Chunked(_) => panic!("chunked body"),
        }
    }

    pub fn body(&self) -> String {
        match &self.content {
            Content::Full(body) => body.read().unwrap().clone(),
            Content::Chunked(state) => {
                let chunk_state = state.read().unwrap();

                if !chunk_state.complete {
                    panic!("body(): incomplete chunks")
                }

                chunk_state
                    .chunks
                    .iter()
                    .map(|c| c.0.clone())
                    .collect::<Vec<String>>()
                    .join("")
                    .to_string()
            }
        }
    }

    pub fn stream(&self) -> BodyStream {
        if let Content::Full(_) = self.content {
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
