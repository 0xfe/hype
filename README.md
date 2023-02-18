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
