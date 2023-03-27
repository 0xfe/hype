# hype

hype is a programmable L4/7 load balancer

## To run

Start with debug logging.

```
$ RUST_LOG=debug cargo run --bin hello

# Run with TLS
$ cargo run --bin hello -- -s
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
```

To test:

```
curl --insecure https://localhost:4000
```

## In Progress

-   TLS support
-   Support chunked forwarding (encoding support done)

## TODO

-   Add additional configuration schema for LB handlers
    -   host rewrite
    -   cert files
-   Support X-Forwarded-For
-   Backend healthchecking for balancer targets
-   Don't propagate hop-by-hop-headers
    -   Keep-Alive, Transfer-Encoding, TE, Connection, Trailer, Upgrade, Proxy-Authorization and Proxy-Authenticate
    -   Maybe okay to propagate keep-alive and connection headers.
-   Cache control headers
-   Transfer-Encoding: gzip (note Content-Encoding and Accept-Encoding too)
-   CLI with https://docs.rs/argh/latest/argh/
-   Use templating + #include to make file browser look better
    -   templating engine with https://crates.io/crates/tera
-   gRPC API - https://github.com/hyperium/tonic
-   Implement access log

### Issues / Housekeeping

-   Errors should be derived from error::Error -- see cookie.rs
-   Support multiple headers with the same key
    -   bad request (400) if multiple host headers

### Done

-   Implment "Connection: close", shutdown socket as soon as request is processed.
-   Implement HTTP Keepalive Timeout
-   Implement HTTP Keepalive Max
-   Rewrite host header for load balancer
-   Implement chunked transfer-encoding
-   Implement connection tracking
-   LB handler
-   Fix case sensitivity in headers
-   Move redirection to URL rewriting middleware
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
