# hype is a web server

## To run

Start with debug logging.

```
$ RUST_LOG=debug cargo run
```

## Run tests

```
# Run all tests
$ cargo test

# Run specific tests
$ cargo test parser

# Show standard output
$ cargo test -- --nocapture
```

## TODO

-   implement handler abstraction
-   end-to-end tests with reqwest
-   serve files
-   json handling (is it needed ??) -- serde_json
