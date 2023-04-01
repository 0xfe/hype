use std::sync::Arc;

use futures::StreamExt;
use hype::body::Body;
use tokio::sync::RwLock;

#[tokio::test]
async fn it_works() {
    let body = Arc::new(RwLock::new(Body::new()));
    let mut body = body.write().await;

    body.set_chunked();
    body.push_chunk("foobar");
    body.push_chunk("blah");
    body.end_chunked();

    assert_eq!(body.next().await.unwrap(), "foobar");
    assert_eq!(body.next().await.unwrap(), "blah");
    assert_eq!(body.next().await, None);
}
