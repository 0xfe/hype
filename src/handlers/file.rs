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
    handler::{self, AsyncStream, Handler},
    request::Request,
    response::Response,
    status,
};

pub struct File {
    path: String,
    content_types: HashMap<&'static str, &'static str>,
}

impl File {
    pub fn new(path: String) -> File {
        File {
            path,
            content_types: [
                ("html", "text/html"),
                ("htm", "text/html"),
                ("txt", "text/plain"),
                ("png", "image/png"),
                ("jpg", "image/jpeg"),
                ("jpeg", "image/jpeg"),
            ]
            .into_iter()
            .collect(),
        }
    }

    async fn write_response<'b>(
        w: &mut dyn AsyncStream,
        status: status::Code<'b>,
        content_type: String,
        body: String,
    ) -> io::Result<()> {
        let mut response = Response::new(status::from(status));
        response.set_header("Content-Type".into(), content_type);

        w.write_all(response.set_body(body).serialize().as_bytes())
            .await
    }

    async fn write_dir(
        w: &mut dyn AsyncStream,
        fs_path: String,
        base_fs_path: &String,
        handler_path: &String,
    ) -> Result<(), ()> {
        info!(
            "FS PATH: {} BASE_FS_PATH: {} HANDLER_PATH: {}",
            &fs_path, &base_fs_path, &handler_path
        );
        let mut files = fs::read_dir(&fs_path).await.or(Err(()))?;

        let mut body = String::from("<ul>\n");

        loop {
            let entry = files.next_entry().await.or(Err(()))?;

            if let Some(entry) = &entry {
                let file_name = entry.file_name();
                let file_name = file_name.to_string_lossy();

                let mut pathbuf = PathBuf::new();
                pathbuf.push("/");
                pathbuf.push(handler_path);
                pathbuf.push(&fs_path.strip_prefix(base_fs_path).unwrap().to_string());
                pathbuf.push(file_name.to_string());

                let output = format!(
                    "  <li><a href='{}'>{}</a></li>\n",
                    pathbuf.as_os_str().to_str().unwrap(),
                    file_name
                );
                body = body + &output;
            } else {
                body = body + "<ul>\n";
                break;
            }
        }

        File::write_response(w, status::OK, "text/html".into(), body)
            .await
            .or(Err(()))
    }

    async fn write_file_contents(
        w: &mut dyn AsyncStream,
        path: String,
        content_types: &HashMap<&str, &str>,
    ) -> Result<(), ()> {
        let contents = fs::read_to_string(&path).await.or(Err(()))?;

        let ext = Path::new(&path)
            .extension()
            .unwrap_or(&OsStr::new("txt"))
            .to_str()
            .unwrap();

        File::write_response(
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
    ) -> Result<(), handler::Error> {
        let mut path = PathBuf::new();
        path.push(self.path.as_str());
        if !r.path().is_empty() {
            path.push(&r.path()[1..]);
        }

        let path = path.to_str().ok_or(handler::Error::Failed(
            "could not parse request path".into(),
        ))?;

        info!("Serving file: {}", path);
        let metadata = fs::metadata(path).await.or(Err(handler::Error::Failed(
            "could not fetch file metadata".to_string(),
        )))?;

        let path = String::from(path);

        let default_path = String::new();
        let handler_path = r.handler_path.as_ref().unwrap_or(&default_path);
        let handler_path = handler_path
            .strip_prefix('/')
            .unwrap_or(&handler_path)
            .to_string();

        if metadata.is_dir() {
            File::write_dir(w, path, &self.path, &handler_path)
                .await
                .or(Err(handler::Error::Failed(
                    "could not list directory".into(),
                )))?;
        } else {
            File::write_file_contents(w, path, &self.content_types)
                .await
                .or(Err(handler::Error::Failed("could not open file".into())))?;
        }

        Ok(())
    }
}

#[async_trait]
impl Handler for File {
    async fn handle(&self, r: &Request, w: &mut dyn AsyncStream) -> Result<(), handler::Error> {
        let result = self.handle_path(r, w).await;

        if let Err(err) = &result {
            File::write_response(
                w,
                status::NOT_FOUND,
                "text/plain".into(),
                format!("404 NOT FOUND - {:?}", err),
            )
            .await
            .or(Err(handler::Error::Failed(
                "could not write to stream".into(),
            )))?;

            return result;
        }

        Ok(())
    }
}
