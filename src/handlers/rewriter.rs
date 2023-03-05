use async_trait::async_trait;
use regex::Regex;
use tokio::io::AsyncWriteExt;

use crate::{
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct Rewriter {
    url_match_re: Regex,
    substitution: String,
}

impl Rewriter {
    pub fn new(
        url_match_re: impl Into<String>,
        substitution: impl Into<String>,
    ) -> Result<Self, String> {
        let url_match_re = url_match_re.into();

        return Ok(Self {
            url_match_re: Regex::new(url_match_re.clone().as_str())
                .or(Err(format!("bad regex: {}", url_match_re)))?,
            substitution: substitution.into(),
        });
    }
}

#[async_trait]
impl Handler for Rewriter {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut response = Response::new(status::from(status::MOVED_PERMANENTLY));
        let path = r.abs_path();
        let location = self.url_match_re.replace(path.as_str(), &self.substitution);

        if location != path {
            response.set_header("Location", location);
            let buf = response.serialize();
            w.write_all(buf.as_bytes()).await.unwrap();
        }

        Ok(handler::Ok::Done)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn trailing_slashes() {
        let r = r##"POST /foo/bar HTTP/1.1
Host: localhost:4000
Content-Type: application/x-www-form-urlencoded
Content-Length: 23

merchantID=2003&foo=bar"##;

        let rewriter = Rewriter::new("(.*)([^/]$)", "$1$2/").unwrap();
        let request = Request::from(r, "http://localhost").unwrap();
        let mut stream: Vec<u8> = vec![];

        rewriter.handle(&request, &mut stream).await.unwrap();
        println!("{:?}", String::from_utf8_lossy(&stream));

        assert_eq!(
            std::str::from_utf8(&stream).unwrap(),
            "HTTP/1.1 301 Moved Permanently\r\nLocation: /foo/bar/\r\n\r\n"
        );
    }

    #[tokio::test]
    async fn trailing_slashes_nop() {
        let r = r##"POST /foo/bar/ HTTP/1.1
Host: localhost:4000
Content-Type: application/x-www-form-urlencoded
Content-Length: 23

merchantID=2003&foo=bar"##;

        let rewriter = Rewriter::new("(.*)([^/]$)", "$1$2/").unwrap();
        let request = Request::from(r, "http://localhost").unwrap();
        let mut stream: Vec<u8> = vec![];

        rewriter.handle(&request, &mut stream).await.unwrap();
        println!("{:?}", String::from_utf8_lossy(&stream));

        assert_eq!(std::str::from_utf8(&stream).unwrap(), "");
    }
}
