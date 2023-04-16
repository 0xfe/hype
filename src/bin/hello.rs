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

async fn hello1(_: Request, _: ()) -> Result<impl Into<String>, handler::Error> {
    Ok("Hello world!")
}

async fn hello2(_: Request, _: ()) -> Result<Response, handler::Error> {
    Ok(Response::new(status::OK).with_body("yooo!"))
}

async fn hello3(r: Request, _: ()) -> Result<String, handler::Error> {
    Ok(format!(
        "Hello, {}: {}!",
        r.path(),
        r.query_params()
            .get("name")
            .unwrap_or(&String::from("world"))
    ))
}

async fn hello4(r: Request, _: ()) -> Result<String, handler::Error> {
    Ok(format!(
        "Hello, {}!",
        r.params
            .get("name")
            .ok_or(handler::Error::Status(status::NOT_FOUND.into()))?
    ))
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

    // Hello world with inline async block returning String.
    server.route(
        "/hello",
        handlers::service(|_, _: ()| async move { Ok("boo!") }),
    );

    // Hello world with function returning string.
    server.route("/hello1", handlers::service(hello1));

    // Hello world with function returning Response.
    server.route("/hello2", handlers::service(hello2));

    // Hello world parsing URL parametrs
    server.route("/hello3", handlers::service(hello3));

    // Hello world with path matcher
    server.route("/hello4/:name", handlers::service(hello4));
    server.route_default(NotFoundHandler());
    server.start().await.unwrap();
}
