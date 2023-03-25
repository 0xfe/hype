// Set default log level to info. To change, set RUST_LOG as so:
//
//    $ RUST_LOG=debug cargo run

pub fn init() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
}
