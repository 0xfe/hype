use std::sync::Arc;

use futures::StreamExt;
use hype::body::Body;

#[tokio::test]
async fn it_works() {
    let mut body = Body::new();

    body.set_chunked();
    body.push_chunk("foobar".into());
    body.push_chunk("blah".into());
    body.end_chunked();

    let mut stream = body.chunk_stream();

    assert_eq!(stream.next().await.unwrap(), "foobar".as_bytes());
    assert_eq!(stream.next().await.unwrap(), "blah".as_bytes());
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
            body_writer.push_chunk("foobar".into());
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
            body_writer.push_chunk("foobar".into());
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

    while let Some(_) = stream2.next().await {
        count += 1;
    }

    assert_eq!(count, 20);
}

#[tokio::test]
async fn it_works_with_raw_stream() {
    let mut body = Body::new();
    body.set_chunked();

    let body_writer = Arc::new(body);
    let body_reader = Arc::clone(&body_writer);

    tokio::spawn(async move {
        for _ in 0..3 {
            body_writer.push_chunk("foobar".into());
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        body_writer.end_chunked();
    });

    let mut stream = body_reader.raw_stream();

    let data = stream.next().await.unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "6\r\nfoobar\r\n");

    let data = stream.next().await.unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "6\r\nfoobar\r\n");

    let data = stream.next().await.unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "6\r\nfoobar\r\n");

    let data = stream.next().await.unwrap();
    assert_eq!(String::from_utf8(data).unwrap(), "0\r\n\r\n");
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

#[tokio::test]
async fn read_all() {
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

    let data = body_reader.content().await;

    assert_eq!(data, "foobar 0foobar 1foobar 2foobar 3foobar 4".as_bytes());
}

#[tokio::test]
async fn read_all_chunks() {
    let mut body = Body::new();
    body.set_chunked();

    let body_writer = Arc::new(body);
    let body_reader = Arc::clone(&body_writer);

    tokio::spawn(async move {
        for i in 0..5 {
            body_writer.push_chunk(format!("foobar {i}").into());
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        body_writer.end_chunked();
    });

    let data = body_reader.content().await;

    assert_eq!(data, "foobar 0foobar 1foobar 2foobar 3foobar 4".as_bytes());
}
