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
    /// whether or not to jump
    #[argh(option, short = 'p', default = "80")]
    port: u16,

    /// how high to go
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
        client.set_secure(&args.host);
    }

    let mut client = client.connect().await.unwrap();

    let mut request = Request::new();
    request.set_method(hype::request::Method::GET);
    request.set_path("/");

    let response = client.send_request(&request).await.unwrap();
    info!("Response: {:?}", response);
}
