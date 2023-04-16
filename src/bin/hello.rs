#[macro_use]
extern crate log;

use argh::FromArgs;

use hype::handler::{self};
use hype::response::Response;
use hype::{handlers, status};
use hype::{handlers::status::NotFoundHandler, request::Request, server::Server};

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

async fn hello(r: Request, _: ()) -> Result<String, handler::Error> {
    Ok(format!(
        "Hello, {}: {}!",
        r.path(),
        r.query_params()
            .get("name")
            .unwrap_or(&String::from("world"))
    ))
}

async fn hello2(_: Request, _: ()) -> Result<Response, handler::Error> {
    let r = Response::new(status::from(status::OK)).with_body("yooo!");
    Ok(r)
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

    server
        .route(
            "/boo",
            handlers::service(|_, _: ()| async move { Ok("boo!") }),
        )
        .await;
    server.route("/hello", handlers::service(hello)).await;
    server.route("/hello2", handlers::service(hello2)).await;
    server.route_default(NotFoundHandler());
    server.start().await.unwrap();
}
