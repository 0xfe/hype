use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

#[derive(Debug)]
pub struct FileHandlerParams {
    pub fs_path: String,
}

#[derive(Debug)]
pub enum Handler {
    File(FileHandlerParams),
    App(String),
}

#[derive(Debug)]
pub struct Route {
    pub location: String,
    pub handler: Handler,
}

#[derive(Debug)]
pub struct Config {
    pub routes: Vec<Route>,
}

impl Config {
    pub fn from_str(config_str: impl AsRef<str>) -> Result<Self, String> {
        let mut config = Config { routes: vec![] };

        for document in Deserializer::from_str(config_str.as_ref()) {
            let value = Value::deserialize(document).unwrap();
            let routes = value.get("routes").ok_or("missing routes".to_string())?;

            if !routes.is_sequence() {
                return Err("expected list of routes".to_string());
            }

            for r in routes.as_sequence().unwrap() {
                let location = r
                    .get("location")
                    .ok_or("missing route parameter".to_string())?;
                let handler = r
                    .get("handler")
                    .ok_or("missing handler parameter".to_string())?;
                let fs_path = r
                    .get("fs_path")
                    .ok_or("missing fs_path parameter for file handler".to_string())?;

                if !location.is_string() {
                    return Err("location should be a string".into());
                }
                if !handler.is_string() || handler.as_str().unwrap() != "file" {
                    return Err("handler should be one of 'file' or 'path'".into());
                }
                if !fs_path.is_string() {
                    return Err("path should be a valid filesystem path".into());
                }

                config.routes.push(Route {
                    location: location.as_str().unwrap().to_string(),
                    handler: Handler::File(FileHandlerParams {
                        fs_path: fs_path.as_str().unwrap().to_string(),
                    }),
                })
            }
        }

        Ok(config)
    }
}
