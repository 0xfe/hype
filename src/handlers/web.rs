use std::{
    collections::HashMap,
    ffi::OsStr,
    path::{Path, PathBuf},
};

use async_trait::async_trait;
use tokio::{
    fs,
    io::{self, AsyncWriteExt},
};

use crate::{
    config, content_types,
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct Web {
    base_fs_path: String,
    content_types: HashMap<&'static str, &'static str>,
    index_files: Vec<String>,
    hosts: Vec<String>,
    trailing_slashes: bool,
}

impl Web {
    pub fn new(base_fs_path: String) -> Self {
        Web {
            base_fs_path,
            content_types: content_types::BY_EXT.clone(),
            index_files: vec!["index.html".into(), "index.htm".into()],
            hosts: vec![],
            trailing_slashes: true,
        }
    }

    pub fn from(params: &config::WebHandlerParams) -> Self {
        Web {
            base_fs_path: params.webroot.clone(),
            content_types: content_types::BY_EXT.clone(),
            index_files: vec![params.index.clone()],
            hosts: params.hosts.clone(),
            trailing_slashes: params.trailing_slashes,
        }
    }

    async fn write_response<'b>(
        w: &mut dyn AsyncStream,
        status: status::Code<'b>,
        content_type: String,
        body: String,
    ) -> io::Result<()> {
        let mut response = Response::new(status::from(status));
        response.set_header("Content-Type", content_type);

        w.write_all(response.set_body(body).serialize().as_bytes())
            .await
    }

    async fn write_file_contents(
        w: &mut dyn AsyncStream,
        path: impl AsRef<str>,
        content_types: &HashMap<&str, &str>,
    ) -> Result<(), ()> {
        let contents = fs::read_to_string(path.as_ref()).await.or(Err(()))?;

        let ext = Path::new(path.as_ref())
            .extension()
            .unwrap_or(&OsStr::new("txt"))
            .to_str()
            .unwrap();

        Self::write_response(
            w,
            status::OK,
            content_types.get(ext).unwrap_or(&"txt").to_string(),
            contents,
        )
        .await
        .or(Err(()))
    }

    async fn handle_path(
        &self,
        r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let mut abs_fs_path = PathBuf::new();
        abs_fs_path.push(self.base_fs_path.as_str());

        if !r.path().is_empty() {
            abs_fs_path.push(&r.path()[1..]);
        }

        let abs_fs_path = abs_fs_path.to_str().ok_or(handler::Error::Failed(
            "could not parse request path".into(),
        ))?;

        info!("Serving FS path {} at location {}", abs_fs_path, r.path());
        let metadata = fs::metadata(abs_fs_path)
            .await
            .or(Err(handler::Error::Failed(
                "could not fetch file metadata".to_string(),
            )))?;

        let abs_fs_path = String::from(abs_fs_path);

        if let Some(host) = r.host() {
            if !self.hosts.is_empty() && !self.hosts.contains(host) {
                return Err(handler::Error::Failed(format!(
                    "host {} does not match: {:?}",
                    host, self.hosts
                )));
            }
        }

        if metadata.is_dir() {
            if self.trailing_slashes && !r.abs_path().ends_with("/") {
                info!("Redirecting {} to {}", r.abs_path(), r.abs_path() + "/");
                return Ok(handler::Ok::Redirect(r.abs_path() + "/"));
            }

            for index in &self.index_files {
                let path = PathBuf::from(&abs_fs_path).join(index);

                if Path::new(&path).exists() {
                    Self::write_file_contents(
                        w,
                        path.as_os_str().to_str().unwrap(),
                        &self.content_types,
                    )
                    .await
                    .or(Err(handler::Error::Failed("could not open file".into())))?;
                    return Ok(handler::Ok::Done);
                }
            }

            return Err(handler::Error::Failed("no index file in path".into()));
        } else {
            Self::write_file_contents(w, abs_fs_path, &self.content_types)
                .await
                .or(Err(handler::Error::Failed("could not open file".into())))?;
        }

        Ok(handler::Ok::Done)
    }
}

#[async_trait]
impl Handler for Web {
    async fn handle(
        &self,
        r: &Request,
        w: &mut dyn AsyncStream,
    ) -> Result<handler::Ok, handler::Error> {
        let result = self.handle_path(r, w).await;

        return match result {
            Ok(handler::Ok::Redirect(to)) => {
                let mut response = Response::new(status::from(status::MOVED_PERMANENTLY));
                response.set_header("Location", to);
                w.write_all(response.serialize().as_bytes()).await.or(Err(
                    handler::Error::Failed("could not write to stream".into()),
                ))?;
                Ok(handler::Ok::Done)
            }
            Ok(ok) => Ok(ok),
            Err(handler::Error::Failed(msg)) => {
                Web::write_response(
                    w,
                    status::NOT_FOUND,
                    "text/plain".into(),
                    format!("404 NOT FOUND - {}", msg),
                )
                .await
                .or(Err(handler::Error::Failed(
                    "could not write to stream".into(),
                )))?;

                Err(handler::Error::Failed(msg))
            }
        };
    }
}
