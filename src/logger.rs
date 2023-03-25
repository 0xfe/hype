// Set default log level to info. To change, set RUST_LOG as so:
//
//    $ RUST_LOG=debug cargo run

pub fn init() {
    // We use try_init here so it can by run by tests.
    _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
}
