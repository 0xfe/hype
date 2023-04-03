#[macro_use]
extern crate log;

use argh::FromArgs;
use async_trait::async_trait;

use hype::{
    handler::{self, AsyncWriteStream, Handler},
    request::Request,
    response::Response,
    server::Server,
    status::{self},
};
use tokio::io::AsyncWriteExt;

#[derive(FromArgs)]
/// Reach new heights.
struct Args {
    /// server hostname or IP
    #[argh(option, short = 'h', default = "String::from(\"localhost\")")]
    host: String,

    /// server port
    #[argh(option, short = 'p', default = "4000")]
    port: u16,

    /// enable TLS
    #[argh(switch, short = 's')]
    secure: bool,

    /// TLS cert file
    #[argh(option, default = "String::from(\"localhost.crt\")")]
    cert_file: String,

    /// TLS key file
    #[argh(option, default = "String::from(\"localhost.key\")")]
    key_file: String,
}

struct MyHandler {}

#[async_trait]
impl Handler for MyHandler {
    async fn handle(
        &self,
        _r: &Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::OK));
        response.set_body("<html>Hello world!</html>\n".into());

        let buf = response.serialize();
        w.write_all(buf.as_bytes()).await.unwrap();
        Ok(handler::Ok::Done)
    }
}

#[tokio::main]
async fn main() {
    // Set default log level to info. To change, set RUST_LOG as so:
    //
    //    $ RUST_LOG=debug cargo run
    hype::logger::init();
    let args: Args = argh::from_env();

    info!("Starting hype...");
    let mut server = Server::new(args.host, args.port);

    if args.secure {
        server.enable_tls(args.cert_file.into(), args.key_file.into());
    }

    server.route_default(Box::new(MyHandler {}));
    server.start().await.unwrap();
}
