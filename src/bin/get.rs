/// Run this with:
///
/// ```
/// $ cargo run --bin get google.com
/// $ cargo run --bin get google.com -s -p 443
/// ```
#[macro_use]
extern crate log;
use argh::FromArgs;
use hype::{client::Client, request::Request};

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// server port
    #[argh(option, short = 'p', default = "80")]
    port: u16,

    /// enable TLS
    #[argh(switch, short = 's')]
    secure: bool,

    #[argh(positional)]
    host: String,
}

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    hype::logger::init();

    let args: Args = argh::from_env();

    info!("Starting hype...");
    let mut client = Client::new(format!("{}:{}", &args.host, &args.port));
    if args.secure {
        client.enable_tls(&args.host);
    }

    let mut client = client.connect().await.unwrap();
    let request = Request::new(hype::request::Method::GET, "/");

    let response = client.send_request(&request).await.unwrap();
    info!("Headers:\n {:?}", response.serialize_headers());
    info!("Body:\n {:?}", response.content().await.unwrap());
}
