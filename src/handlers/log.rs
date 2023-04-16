use crate::{
    handler::{self},
    request::Request,
};

use super::service::{service, ServiceHandler};

async fn log_handler(r: Request, _: ()) -> Result<handler::Action, handler::Error> {
    info!("Request {}", r.url.as_ref().unwrap());
    Ok(handler::Action::Next)
}

pub fn log() -> ServiceHandler<handler::Action, ()> {
    service(log_handler)
}
