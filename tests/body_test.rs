use std::sync::Arc;

use futures::StreamExt;
use hype::body::{Body, Chunk};

#[tokio::test]
async fn it_works() {
    let mut body = Body::new();

    body.set_chunked();
    body.push_chunk("foobar");
    body.push_chunk("blah");
    body.end_chunked();

    let mut stream = body.stream();

    assert_eq!(stream.next().await.unwrap(), Chunk("foobar".into()));
    assert_eq!(stream.next().await.unwrap(), Chunk("blah".into()));
    assert_eq!(stream.next().await, None);
}

#[tokio::test]
async fn it_works_with_waker() {
    let mut body = Body::new();
    body.set_chunked();

    let body_writer = Arc::new(body);
    let body_reader = Arc::clone(&body_writer);

    tokio::spawn(async move {
        for _ in 0..10 {
            body_writer.push_chunk("foobar");
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        body_writer.end_chunked();
    });

    let mut stream = body_reader.stream();
    let mut count = 0;
    while let Some(chunk) = stream.next().await {
        println!("got chunk: {}", chunk.0);
        count += 1;
    }

    assert_eq!(count, 10);
}
