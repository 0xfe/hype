/// This file implements the Body type, which is used to store the body of HTTP requests and
/// responses. It supports chunked encoding, and can be used to stream data to and from the
/// server.
use std::{
    error, fmt,
    pin::Pin,
    sync::{Arc, RwLock},
    task::{Context, Poll, Waker},
};

use futures::{Stream, StreamExt};

#[derive(Debug)]
pub enum BodyError {
    IncompleteBody,
    ContentTooLong(usize, usize),
    UTF8DecodeFailed(String),
}

impl fmt::Display for BodyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Self::IncompleteBody => "incomplete body".to_string(),
            Self::ContentTooLong(want, got) => {
                format!("content too long, want: {}, got {}", want, got)
            }
            Self::UTF8DecodeFailed(err) => format!("UTF-8 decode failed: {}", err),
        };

        write!(f, "BodyError: {}", e)
    }
}

impl error::Error for BodyError {}

/// We have two body types, based on their encoding: Chunked and Full.
#[derive(Debug, Clone)]
enum Content {
    Chunked(Arc<RwLock<ChunkState>>),
    Full(Arc<RwLock<ContentState>>),
}

#[derive(Debug, Clone)]
struct ChunkState {
    // chunked body
    chunks: Vec<Vec<u8>>,

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

#[derive(Debug, Clone)]
struct ContentState {
    content: Vec<u8>,
    expected_length: usize,
    wakers: Vec<Waker>,
}

impl ContentState {
    fn new() -> Self {
        Self {
            content: vec![],
            expected_length: 0,
            wakers: vec![],
        }
    }
}

impl<T: Into<String>> From<T> for ContentState {
    fn from(val: T) -> Self {
        let val = val.into();

        Self {
            content: val.as_bytes().to_vec(),
            expected_length: val.len(),
            wakers: vec![],
        }
    }
}

#[derive(Debug, Clone)]
pub struct Body {
    content: Content,
}

impl<T: Into<String>> From<T> for Body {
    fn from(val: T) -> Self {
        Self {
            content: Content::Full(Arc::new(RwLock::new(ContentState::from(val)))),
        }
    }
}

impl Body {
    pub fn new() -> Self {
        Self {
            content: Content::Full(Arc::new(RwLock::new(ContentState::new()))),
        }
    }

    pub fn set_chunked(&mut self) {
        self.content = Content::Chunked(Arc::new(RwLock::new(ChunkState::new())));
    }

    pub fn set_content_length(&mut self, length: usize) {
        match &self.content {
            Content::Full(state) => state.write().unwrap().expected_length = length,
            Content::Chunked(_) => panic!("chunked body"),
        }
    }

    pub fn chunked(&self) -> bool {
        if let Content::Chunked(_) = self.content {
            return true;
        }

        false
    }

    pub fn push_chunk(&self, chunk: Vec<u8>) {
        match &self.content {
            Content::Full(_) => panic!("not chunked"),
            Content::Chunked(state) => {
                // Notify async stream wakers that there's a new chunk
                let mut wakers = vec![];
                {
                    let mut chunk_state = state.write().unwrap();
                    chunk_state.chunks.push(chunk);
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

    /// Returns true if the body is complete.
    pub fn complete(&self) -> bool {
        match &self.content {
            Content::Full(state) => {
                let state = state.read().unwrap();
                state.content.len() >= state.expected_length
            }
            Content::Chunked(state) => state.read().unwrap().complete,
        }
    }

    /// Append bytes to the body. Returns true if the body is complete.
    pub fn append(&self, buf: &[u8]) -> Result<bool, BodyError> {
        match &self.content {
            Content::Full(state) => {
                let mut wakers = vec![];
                let mut done = false;
                {
                    let mut state = state.write().unwrap();
                    state.content.extend(buf);
                    if state.content.len() > state.expected_length {
                        state.content = state.content[..state.expected_length].to_vec();
                    }

                    if state.content.len() == state.expected_length {
                        done = true;
                    }

                    std::mem::swap(&mut wakers, &mut state.wakers);
                }
                wakers.iter().for_each(|w| w.wake_by_ref());
                Ok(done)
            }
            Content::Chunked(_) => panic!("chunked body"),
        }
    }

    /// Return as much of the body as is available.
    pub fn try_content(&self) -> Vec<u8> {
        match &self.content {
            Content::Full(body) => body.read().unwrap().content.clone(),
            Content::Chunked(state) => {
                let chunk_state = state.read().unwrap();
                chunk_state.chunks.concat()
            }
        }
    }

    /// Return the full body as a string, blocking until it's complete.
    pub async fn content(&self) -> Vec<u8> {
        self.stream().concat().await
    }

    pub fn content_stream(&self) -> ContentStream {
        let content_state;
        if let Content::Full(state) = &self.content {
            content_state = Arc::clone(state);
        } else {
            panic!("content_stream(): chunked content")
        }

        ContentStream {
            state: content_state,
            current_pos: 0,
        }
    }

    pub fn chunk_stream(&self) -> ChunkStream {
        let chunk_state;
        if let Content::Chunked(state) = &self.content {
            chunk_state = Arc::clone(state);
        } else {
            panic!("chunk_stream(): not chunked")
        }

        ChunkStream {
            done: false,
            raw: false,
            state: chunk_state,
            current_pos: 0,
        }
    }

    pub fn stream(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send + Sync>> {
        if let Content::Full(_) = &self.content {
            Box::pin(self.content_stream())
        } else {
            Box::pin(self.chunk_stream())
        }
    }

    pub fn raw_stream(&self) -> Pin<Box<dyn Stream<Item = Vec<u8>> + Send + Sync>> {
        if let Content::Full(_) = &self.content {
            Box::pin(self.content_stream())
        } else {
            let mut stream = self.chunk_stream();
            stream.raw = true;
            Box::pin(stream)
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ChunkStream {
    raw: bool,
    done: bool,
    state: Arc<RwLock<ChunkState>>,
    current_pos: usize,
}

impl Stream for ChunkStream {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Vec<u8>>> {
        let mut current_chunk = None;
        let mut done = self.done;
        let mut return_val = None;

        {
            let mut chunk_state = self.state.write().unwrap();
            if self.current_pos >= chunk_state.chunks.len() {
                if !chunk_state.complete {
                    // More chunks are coming, return Pending
                    chunk_state.wakers.push(cx.waker().clone());
                    return_val = Some(Poll::Pending);
                } else if self.raw && !done {
                    // No more chunks, send closing '0' chunk
                    done = true;
                    let chunk = vec![b'0', b'\r', b'\n', b'\r', b'\n'];
                    return_val = Some(Poll::Ready(Some(chunk)));
                } else {
                    // Closing chunk sent, close stream
                    return_val = Some(Poll::Ready(None));
                }
            } else {
                current_chunk = Some(chunk_state.chunks[self.current_pos].clone());
            }
        }

        if done {
            self.done = true;
        }

        if let Some(current_chunk) = current_chunk {
            self.current_pos += 1;
            let mut chunk: Vec<u8>;
            if self.raw {
                chunk = format!("{:x}", current_chunk.len()).as_bytes().to_vec();
                chunk.extend([b'\r', b'\n']);
                chunk.extend(current_chunk);
                chunk.extend([b'\r', b'\n']);
            } else {
                chunk = current_chunk;
            }

            Poll::Ready(Some(chunk))
        } else {
            return_val.unwrap()
        }
    }
}

pub struct ContentStream {
    state: Arc<RwLock<ContentState>>,
    current_pos: usize,
}

impl Stream for ContentStream {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Vec<u8>>> {
        let content;

        {
            let mut state = self.state.write().unwrap();
            if self.current_pos >= state.content.len() {
                if state.content.len() != state.expected_length {
                    state.wakers.push(cx.waker().clone());
                    return Poll::Pending;
                }
                return Poll::Ready(None);
            }

            content = state.content[self.current_pos..].to_vec();
        }

        self.current_pos += content.len();
        Poll::Ready(Some(content))
    }
}
