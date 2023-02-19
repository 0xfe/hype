#![allow(dead_code)]

use std::fmt;

use crate::parser::Request;

#[derive(Debug)]
pub enum Error {
    Failed,
}

pub trait HandlerFnT: Fn(&Request) -> Result<(), Error> + Send + Sync {}
impl<F> HandlerFnT for F where F: Fn(&Request) -> Result<(), Error> + Send + Sync {}
pub type HandlerFn = dyn HandlerFnT<Output = Result<(), Error>>;

impl fmt::Debug for HandlerFn {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "HandlerFn").unwrap();
        Ok(())
    }
}

#[derive(Debug)]
pub struct Handler {
    method: String,
    func: Box<HandlerFn>,
}

impl Handler {
    pub fn new(method: String, func: Box<HandlerFn>) -> Handler {
        Handler { method, func }
    }

    pub fn call(&self, request: &Request) -> Result<(), Error> {
        let f = &self.func;
        f(request)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        fn boo(r: &Request) -> Result<(), Error> {
            println!("boo: {:?}", r);
            Ok(())
        }

        let h = Handler::new("GET".to_string(), Box::new(&boo));
        h.call(&Request::new()).unwrap();
    }
}
