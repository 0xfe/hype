#![allow(dead_code)]

use crate::parser::Request;

#[derive(Debug)]
pub enum Error {
    Failed,
}

#[derive(Debug)]
pub struct Handler<F: FnOnce(&Request) -> Result<(), Error>> {
    method: String,
    func: F,
}

impl<F: FnOnce(&Request) -> Result<(), Error>> Handler<F> {
    pub fn new(method: String, func: F) -> Handler<F> {
        Handler { method, func }
    }

    pub fn call(self, request: &Request) -> Result<(), Error> {
        let f = self.func;
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

        let h = Handler::new("GET".to_string(), boo);
        h.call(&Request::new()).unwrap();
    }
}
