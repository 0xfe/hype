# hype

hype is a programmable L4/7 load balancer

## Hello World!

See [hello.rs](https://github.com/0xfe/hype/blob/main/src/bin/hello.rs) for an example.

```rust
async fn hello1(_: Request, _: ()) -> Result<impl Into<String>, handler::Error> {
    Ok("Hello world!")
}

async fn hello2(_: Request, _: ()) -> Result<Response, handler::Error> {
    let r = Response::new(status::from(status::OK)).with_body("yooo!");
    Ok(r)
}

async fn hello3(r: Request, _: ()) -> Result<String, handler::Error> {
    Ok(format!(
        "Hello, {}!",
        r.params.get("name").unwrap_or(&String::from("world"))
    ))
}

#[tokio::main]
async fn main() {
    hype::logger::init();
    let mut server = Server::new("localhost", 4000);

    // Hello world with inline async block returning String.
    server.route(
        "/hello",
        handlers::service(|_, _: ()| async move { Ok("boo!") }),
    );

    // Hello world with function returning string.
    server.route("/hello1", handlers::service(hello1));

    // Hello world with function returning Response.
    server.route("/hello2", handlers::service(hello2));

    // Hello world with path matcher
    server.route("/hello3/:name", handlers::service(hello4));
    server.start().await.unwrap();
}
```

## Hello Loadbalancer!

```rust
fn build_backend(backend: &lbconfig::Backend) -> HttpBackend {
    let mut b = HttpBackend::new(format!("{}:{}", backend.host, backend.port));
    if backend.enable_tls {
        b.enable_tls(backend.host.clone());
    }
    b
}

#[tokio::main]
async fn main() {
    hype::logger::init();

    let mut server = Server::new("localhost", 5000);
    server.enable_tls("localhost.crt", "localhost.key");

    let backends: Vec<HttpBackend> = vec![
        HttpBackend::new("localhost:8000")
        HttpBackend::new("localhost:8001")
        HttpBackend::new("localhost:8002")
    ];

    // Create a new load balancer which uses the RoundRobin strategy, and
    // rewrites the Host: header.
    let mut balancer = Http::new(backends, RRPicker::new());
    balancer.rewrite_header("host", "localhost:5000");

    // Create a handler for the balancer and attach it to /balancer. Requests
    // to /balancer will hit one of the backends.
    server.route("/balancer", hype::handlers::Lb::new(balancer));
    server.start().await.unwrap();
}
```

## Running binaries in bin/

Start with debug logging.

```

$ RUST_LOG=debug cargo run --bin hello

# Run with TLS

$ cargo run --bin hello -- -s

# Run balancer

$ vi lbconfig.yaml
$ cargo run --bin balancer

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

# To see logs (via `info!`, `debug!` etc.) in tests, add the following line to the test.

hype::logger::init()

# Then run the tests with the `-- --nocapture` flag, and optionally set the `RUST_LOG=debug` env var.

# Build docs

$ cargo doc

```

## Generate TLS key and cert file for localhost

```

$ openssl genrsa -out localhost.key 2048
$ openssl req -new -x509 -sha256 -key localhost.key -out localhost.crt -days 3650

# To test:

curl --insecure https://localhost:4000

```

## In Progress

-   [x] Simplified handler using service/state API
-   [ ] Improve matcher rules system
    -   [ ] Ignore trailing slashes for prefix matches
    -   [x] Support longest matching path
    -   [x] Support positional parameters
-   [ ] REST command server and CLI with https://docs.rs/argh/latest/argh/
    -   [x] POST /admin/backends
    -   [x] GET /admin/backends/:id
-   [x] Make it easy to share middleware across routes
-   [ ] Make Request::clone() cheaper for service API

## TODO

-   [ ] Implement gzip transfer encoding
-   [ ] Implement wildcard host matching and rewriting
-   [ ] Support path override in LB configuration
-   [ ] Support X-Forwarded-For
-   [ ] Backend healthchecking for balancer targets
-   [ ] Don't propagate hop-by-hop-headers
    -   Keep-Alive, Transfer-Encoding, TE, Connection, Trailer, Upgrade, Proxy-Authorization and Proxy-Authenticate
    -   Maybe okay to propagate keep-alive and connection headers.
-   [ ] Cache control headers
-   [ ] Use templating + #include to make file browser look better
    -   templating engine with https://crates.io/crates/tera

## DONE

-   [x] Build balancer end-to-end unit tests
-   [x] Implement multimap-based headers and rewriting
-   [x] Support multiple headers with the same key
    -   bad request (400) if multiple host headers
-   [ ] Use https://github.com/dtolnay/thiserror and anyhow::Error for error management
-   [x] Support streaming forwarding (encoding support done)
    -   [x] remove pub fields in request and response
    -   [x] factor Body into request and response
    -   [x] factor Body into parser
    -   [x] support streaming of chunked bodies
        -   [x] futures::Stream implementation for chunked body
        -   [x] return error for body() if chunked and not complete
    -   [x] plumb chunked/content streams through client::Client
    -   [x] plumb chunked streams through server
    -   [x] connect client and server streams through LB
-   [x] use futures::Stream for non-chunked bodies too, forward every read

-   [x] Add additional configuration schema for LB handlers
    -   [x] host rewrite
    -   [x] cert files
-   [x] TLS support
-   [x] Implment "Connection: close", shutdown socket as soon as request is processed.
-   [x] Implement HTTP Keepalive Timeout
-   [x] Implement HTTP Keepalive Max
-   [x] Rewrite host header for load balancer
-   [x] Implement chunked transfer-encoding
-   [x] Implement connection tracking
-   [x] LB handler
-   [x] Fix case sensitivity in headers
-   [x] Move redirection to URL rewriting middleware
-   [x] L7 load balancer
    -   Implemented random backend picker
    -   Implemented Roundrobin picker
    -   Implemented Weighted RR picker
-   [x] Basic loadbalancer framework
-   [x] URL rewrite middleware
-   [x] Config file like lighttpd -- keep it simple, reverse proxy support
    -   access log
    -   index file
    -   server name / host header (with wildcards: \*.example.com)
-   [x] Figure out solution for trailing '/' -- 301 permanent redirect
-   [x] Match Host headers, bad request (400) if multiple headers
-   [x] API to fetch cookies from request header
-   [x] Implement Cookie::try_from(...) to parse from string
-   [x] Implement TryFrom trait for request and cookie
-   [x] API to set cookies in response header -- you can have multiple setcookie headers!
-   [x] Routing: match path components: `/files/*, /pages/*/admin, /files, /files/foo`
-   [x] set content type based on file extension
-   [x] implement handler abstraction
-   [x] URL parsing (use `url` crate) -- https://docs.rs/url/latest/url/
-   [x] API for URL parameters
-   [x] API to get form POST parameters
-   [x] serve files
-   [x] more unit tests

```

```
