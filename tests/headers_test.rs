use hype::headers::Headers;

/// Tests for hype::Headers

#[test]
fn test_headers_new() {
    let headers = Headers::new();
    assert_eq!(headers.len(), 0);
}

#[test]
fn test_headers_add() {
    let mut headers = Headers::new();
    headers.add("Content-Type".to_string(), "text/html".to_string());
    assert_eq!(headers.fields.len(), 1);
    assert_eq!(headers.fields.get("content-type").unwrap().len(), 1);
    assert_eq!(
        headers.fields.get("content-type").unwrap().first().unwrap(),
        "text/html"
    );
}

#[test]
fn test_headers_add_multiple() {
    let mut headers = Headers::new();
    headers.add("Content-Type".to_string(), "text/html".to_string());
    headers.add("Content-Type".to_string(), "text/plain".to_string());
    assert_eq!(headers.fields.len(), 1);
    assert_eq!(headers.fields.get("content-type").unwrap().len(), 2);
    assert_eq!(
        headers.fields.get("content-type").unwrap().first().unwrap(),
        "text/html"
    );
    assert_eq!(
        headers.fields.get("content-type").unwrap().last().unwrap(),
        "text/plain"
    );
}

#[test]
fn test_headers_set() {
    let mut headers = Headers::new();
    headers.add("Content-Type".to_string(), "text/html".to_string());
    headers.set("Content-Type".to_string(), "text/plain".to_string());
    assert_eq!(headers.fields.len(), 1);
    assert_eq!(headers.fields.get("content-type").unwrap().len(), 1);
    assert_eq!(
        headers.fields.get("content-type").unwrap().first().unwrap(),
        "text/plain"
    );
}

#[test]
fn test_headers_remove() {
    let mut headers = Headers::new();
    headers.add("Content-Type".to_string(), "text/html".to_string());
    headers.add("Content-Type".to_string(), "text/plain".to_string());
    headers.remove("Content-Type");
    assert_eq!(headers.fields.len(), 0);
}

#[test]
fn test_headers_get() {
    let mut headers = Headers::new();
    headers.add("Content-Type".to_string(), "text/html".to_string());
    headers.add("Content-Type".to_string(), "text/plain".to_string());
    assert_eq!(headers.fields.len(), 1);
    assert_eq!(headers.fields.get("content-type").unwrap().len(), 2);
    assert_eq!(
        headers.fields.get("content-type").unwrap().first().unwrap(),
        "text/html"
    );
    assert_eq!(
        headers.fields.get("content-type").unwrap().last().unwrap(),
        "text/plain"
    );
}

#[test]
fn test_headers_get_first() {
    let mut headers = Headers::new();
    headers.add("Content-Type".to_string(), "text/html".to_string());
    headers.add("Content-Type".to_string(), "text/plain".to_string());
    assert_eq!(headers.fields.len(), 1);
    assert_eq!(headers.fields.get("content-type").unwrap().len(), 2);
    assert_eq!(
        headers.fields.get("content-type").unwrap().first().unwrap(),
        "text/html"
    );
    assert_eq!(
        headers.fields.get("content-type").unwrap().last().unwrap(),
        "text/plain"
    );
    assert_eq!(headers.get_first("Content-Type").unwrap(), "text/html");
}

#[test]
fn test_headers_serialize() {
    let mut headers = Headers::new();
    headers.add("Content-Type", "text/html");
    headers.add("Content-Type", "text/plain");
    headers.add("Content-Length", "13");
    headers.add("Foo".to_string(), "Bar".to_string());
    headers.add("Foo".to_string(), "Baz".to_string());
    headers.add("Foo".to_string(), "Qux".to_string());
    let serialized = headers.serialize();

    println!("Serialized:\n{}", serialized);
}
