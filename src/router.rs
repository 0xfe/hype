use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    handler::{self, AsyncWriteStream, Handler},
    handlers,
    request::Request,
};

/// This is a wrapper around Handler that allows us easily clone and use them
/// in different routes in multi-threaded contexts.
///
/// Safe to clone.
#[derive(Debug)]
pub struct RouteHandler(Arc<tokio::sync::RwLock<Box<dyn Handler>>>);

impl RouteHandler {
    pub fn new(handler: Box<dyn Handler>) -> RouteHandler {
        RouteHandler(Arc::new(tokio::sync::RwLock::new(handler)))
    }

    pub fn handler(&self) -> Arc<tokio::sync::RwLock<Box<dyn Handler>>> {
        let RouteHandler(handler) = self;
        Arc::clone(handler)
    }
}

impl Clone for RouteHandler {
    fn clone(&self) -> Self {
        RouteHandler(Arc::clone(&self.0))
    }
}

impl<T: Handler + 'static> From<T> for RouteHandler {
    fn from(handler: T) -> RouteHandler {
        RouteHandler::new(Box::new(handler))
    }
}

/// This is the main router struct. It holds a list of routes and their handlers, and
/// finds the best handler for a given request based on the longest matching route.
///
/// Safe to clone.
#[derive(Debug)]
pub struct Router {
    /// List of routes and their handlers.
    handlers: Arc<RwLock<Vec<(Matcher, RouteHandler)>>>,
    pub default_handler: RouteHandler,
}

impl Clone for Router {
    fn clone(&self) -> Self {
        Router {
            handlers: Arc::clone(&self.handlers),
            default_handler: self.default_handler.clone(),
        }
    }
}

impl Router {
    pub fn new() -> Router {
        Router {
            handlers: Arc::new(RwLock::new(Vec::new())),
            default_handler: RouteHandler::new(Box::new(handlers::status::NotFoundHandler())),
        }
    }

    /// Associate a handler with a route.
    pub fn add_route(&self, matcher: Matcher, handler: impl Into<RouteHandler>) {
        let mut handlers = self.handlers.write().unwrap();
        handlers.push((matcher, handler.into()));
        // Sort by matcher length, so that the longest matchers are checked first.
        handlers.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    }

    pub async fn handle(
        &self,
        r: &mut Request,
        w: &mut dyn AsyncWriteStream,
    ) -> Result<handler::Ok, handler::Error> {
        let path = r.url.as_ref().unwrap().path();

        let mut h = self.default_handler.clone();

        // Go through our route handlers ands see if any of them match the request path. The routes
        // are sorted by length, so the first match is the longest match.
        for handler in self.handlers.read().unwrap().iter() {
            if let Some((matched_path, params)) = handler.0.extract_params(&path) {
                r.handler_path = Some(String::from(matched_path.to_string_lossy()));
                r.params = params
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                h = handler.1.clone();
            }
        }

        h.handler().read().await.handle(&r, w).await
    }
}

#[derive(Debug)]
pub struct Matcher {
    pub pattern: PathBuf,
}

/// Matches a URL path to a specified routing pattern. Returns the path
/// that was matched, which can be used to construct an absolute path, or
/// to let handler functions know the relative path from the handler.
impl Matcher {
    pub fn new<T: Into<String>>(pattern: T) -> Matcher {
        Matcher {
            pattern: Path::new(&pattern.into()).into(),
        }
    }

    pub fn len(&self) -> usize {
        self.pattern.components().count()
    }

    pub fn extract_params<'a, T: AsRef<str> + ?Sized>(
        &'a self,
        route: &'a T,
    ) -> Option<(PathBuf, HashMap<&'a str, &'a str>)> {
        let pattern = &self.pattern;
        let mut path_i = Path::new(route.as_ref()).components();
        let mut patt_i = pattern.components();
        let mut params = HashMap::new();

        let mut matched_path = PathBuf::new();

        loop {
            let path = path_i.next();
            let patt = patt_i.next();

            debug!("Matching pattern {:?} against path {:?}", patt, path);

            if let (Some(path), Some(patt)) = (path, patt) {
                let patt = patt.as_os_str().to_str().unwrap();

                if patt.starts_with(":") {
                    let param_name = patt.trim_start_matches(":");
                    let param_value = path.as_os_str().to_str().unwrap();
                    params.insert(param_name, param_value);
                    matched_path.push(path);
                } else if patt == "*" {
                    matched_path.push(path);
                } else if patt == path.as_os_str().to_str().unwrap() {
                    matched_path.push(path);
                } else {
                    return None;
                }
            } else if let (None, Some(patt)) = (path, patt) {
                if patt.as_os_str() != "*" {
                    return None;
                }
            } else if let (Some(_), None) = (path, patt) {
                // we've come this far, return true
                debug!("Matched path: {:?}", matched_path);
                return Some((matched_path, params));
            }

            if path == None && patt == None {
                break;
            }
        }

        debug!("Matched path: {:?}", matched_path);
        Some((matched_path, params))
    }
}
