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

    let mut stream = body.chunk_stream();

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

    let mut stream = body_reader.chunk_stream();
    let mut count = 0;
    while let Some(_) = stream.next().await {
        count += 1;
    }

    assert_eq!(count, 10);
}

// Test multiple chunk streams at the same time
#[tokio::test]
async fn it_works_with_multiple_streams() {
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

    let mut stream1 = body_reader.chunk_stream();
    let mut stream2 = body_reader.chunk_stream();

    let mut count = 0;
    while let Some(_) = stream1.next().await {
        count += 1;
    }

    while let Some(chunk) = stream2.next().await {
        println!("got chunk: {}", chunk.0);
        count += 1;
    }

    assert_eq!(count, 20);
}

// Test content_stream() with a single writer task and multiple reader tasks
#[tokio::test]
async fn it_works_with_content_stream() {
    let mut body = Body::new();
    body.set_content_length(40);

    let body_writer = Arc::new(body);
    let body_reader = Arc::clone(&body_writer);

    tokio::spawn(async move {
        for i in 0..5 {
            body_writer
                .append(format!("foobar {i}").as_bytes())
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let mut stream1 = body_reader.content_stream();

    let mut count = 0;
    while let Some(data) = stream1.next().await {
        println!("got data: {}", String::from_utf8(data).unwrap());
        count += 1;
    }

    assert_eq!(count, 5);
}

// Test content_stream() with multiple writer tasks and multiple reader tasks
#[tokio::test]
async fn it_works_with_content_stream_multiple_writers() {
    let mut body = Body::new();
    body.set_content_length(40);

    let body_writer = Arc::new(body);
    let body_reader = Arc::clone(&body_writer);

    tokio::spawn(async move {
        for i in 0..5 {
            body_writer
                .append(format!("foobar {i}").as_bytes())
                .unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    });

    let mut stream1 = body_reader.content_stream();
    let mut stream2 = body_reader.content_stream();

    let mut count = 0;
    while let Some(data) = stream1.next().await {
        println!("stream1: got data: {}", String::from_utf8(data).unwrap());
        count += 1;
    }

    assert_eq!(count, 5);

    // This should generate all contents in a single call
    while let Some(data) = stream2.next().await {
        println!("stream2: got data: {}", String::from_utf8(data).unwrap());
        count += 1;
    }

    assert_eq!(count, 6);
}
