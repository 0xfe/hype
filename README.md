# hype is a web server from scratch

## Example

```rust
struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        let mut response = Response::new(status::from(status::OK));

        match (r.method, r.url.as_ref().unwrap().path()) {
            (Method::GET | Method::POST, "/") => {
                response.set_body("<html>hi!</html>\n".into());
            }
            _ => {
                response.set_status(status::from(status::NOT_FOUND));
                response.set_body("<html>404 NOT FOUND</html>".into());
            }
        }

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let server = Server::new("127.0.0.1".into(), 4000);
    server.route_default(Box::new(MyHandler {})).await;
}

```

## To run

Start with debug logging.

```
$ RUST_LOG=debug cargo run --bin {app, hello}
```

## Run tests

```
# Run all tests
$ cargo test

# Run specific tests
$ cargo test parser

# Run all tests in file
# cargo test --test request_test

# Show standard output
$ cargo test -- --nocapture
```

## TODO

-   API to fetch cookies from request header
-   CookieJar API
-   json handling (is it needed ??) -- serde_json
-   templating engine with https://crates.io/crates/tera
-   CGI interface

-   Housekeeping
-   end-to-end tests with reqwest

### Done

-   Implement Cookie::try_from(...) to parse from string
-   Implement TryFrom trait for request and cookie
-   API to set cookies in response header -- you can have multiple setcookie headers!
-   Routing: match path components: `/files/*, /pages/*/admin, /files, /files/foo`
-   set content type based on file extension
-   implement handler abstraction
-   URL parsing (use `url` crate) -- https://docs.rs/url/latest/url/
-   API for URL parameters
-   API to get form POST parameters
-   serve files
-   more unit tests
