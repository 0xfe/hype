# hype

-   hype is a web server
-   hype is an L7 load balancer
-   hype is a tiny web framework

## Features implemented so far

-   Handler and middleware API for web apps
-   Simple pattern based request routing
-   L7 loadbalancing with multiple backend selection policies
-   Handler to serve files from directories

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

## TODO

-   LB handler
-   Healthchecking
-   Transfer-Encoding: chunked, gzip (note Content-Encoding and Accept-Encoding too)
-   Implement access log
-   Use templating + #include to make file browser look better
    -   templating engine with https://crates.io/crates/tera
-   gRPC API - https://github.com/hyperium/tonic
-   TLS support
-   json handling (is it needed ??) -- serde_json

### Issues / Housekeeping

-   Fix case sensitivity in headers
-   Move redirection to URL rewriting middleware
-   Errors should be derived from error::Error -- see cookie.rs
-   Support multiple headers with the same key
    -   bad request (400) if multiple host headers
-   Add end-to-end tests with reqwest

### Done

-   L7 load balancer (in progress)
    -   Implemented random backend picker
    -   Implemented Roundrobin picker
    -   Implemented Weighted RR picker
-   Basic loadbalancer framework
-   URL rewrite middleware
-   Config file like lighttpd -- keep it simple, reverse proxy support
    -   access log
    -   index file
    -   server name / host header (with wildcards: \*.example.com)
-   Figure out solution for trailing '/' -- 301 permanent redirect
-   Match Host headers, bad request (400) if multiple headers
-   API to fetch cookies from request header
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
