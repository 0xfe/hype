pub mod file;
pub mod lb;
pub mod log;
pub mod redirect;
pub mod rewriter;
pub mod service;
pub mod status;
pub mod web;

pub use crate::handlers::file::File;
pub use crate::handlers::lb::Lb;
pub use crate::handlers::log::log;
pub use crate::handlers::redirect::Redirect;
pub use crate::handlers::status::NotFoundHandler;
pub use crate::handlers::status::Status;

pub use crate::handlers::service::handler;
pub use crate::handlers::service::service;
