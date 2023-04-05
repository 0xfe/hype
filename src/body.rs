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
    chunks: Vec<String>,

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
                    chunk_state.chunks.push(chunk.into());
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

    pub fn get_chunk(&self, i: usize, waker: Waker) -> Result<String, bool> {
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

    fn get_full_contents(&self, start_pos: usize, waker: Waker) -> Result<Vec<u8>, bool> {
        match &self.content {
            Content::Full(state) => {
                let mut state = state.write().unwrap();
                if start_pos >= state.content.len() {
                    if state.content.len() != state.expected_length {
                        state.wakers.push(waker);
                    }
                    return Err(state.content.len() == state.expected_length);
                }

                Ok(state.content[start_pos..].to_vec())
            }
            Content::Chunked(_) => panic!("chunked body"),
        }
    }

    /// Returns true if the body is complete.
    pub fn full_contents_loaded(&self) -> bool {
        match &self.content {
            Content::Full(state) => {
                let state = state.read().unwrap();
                state.content.len() >= state.expected_length
            }
            Content::Chunked(_) => panic!("chunked body"),
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
    pub fn try_content(&self) -> String {
        match &self.content {
            Content::Full(body) => {
                String::from_utf8_lossy(body.read().unwrap().content.as_slice()).to_string()
            }
            Content::Chunked(state) => {
                let chunk_state = state.read().unwrap();

                chunk_state
                    .chunks
                    .iter()
                    .map(|c| c.clone())
                    .collect::<Vec<String>>()
                    .join("")
                    .to_string()
            }
        }
    }

    /// Return the full body as a string, blocking until it's complete.
    pub async fn content(&self) -> Result<String, BodyError> {
        match &self.content {
            Content::Full(_) => Ok(String::from_utf8(self.content_stream().concat().await)
                .map_err(|e| BodyError::UTF8DecodeFailed(e.to_string()))?),
            Content::Chunked(_) => Ok(String::from_utf8(
                self.chunk_stream()
                    .map(|c| c.as_bytes().to_vec())
                    .concat()
                    .await,
            )
            .map_err(|e| BodyError::UTF8DecodeFailed(e.to_string()))?),
        }
    }

    pub fn content_stream(&self) -> ContentStream {
        if let Content::Chunked(_) = self.content {
            panic!("content_stream(): chunked content")
        }

        ContentStream {
            current_pos: 0,
            body: self,
        }
    }

    pub fn chunk_stream(&self) -> ChunkStream {
        if let Content::Full(_) = self.content {
            panic!("chunk_stream(): not chunked")
        }

        ChunkStream {
            current_chunk: 0,
            body: self,
        }
    }
}

pub struct ChunkStream<'a> {
    current_chunk: usize,
    body: &'a Body,
}

impl<'a> Stream for ChunkStream<'a> {
    type Item = String;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<String>> {
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

pub struct ContentStream<'a> {
    current_pos: usize,
    body: &'a Body,
}

impl<'a> Stream for ContentStream<'a> {
    type Item = Vec<u8>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Vec<u8>>> {
        let contents = self
            .body
            .get_full_contents(self.current_pos, cx.waker().clone());

        match contents {
            Ok(contents) => {
                self.current_pos += contents.len();
                Poll::Ready(Some(contents))
            }
            Err(true) => Poll::Ready(None),
            Err(false) => Poll::Pending,
        }
    }
}
