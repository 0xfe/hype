use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

#[derive(Debug)]
pub struct FileHandlerParams {
    pub fs_path: String,
}

#[derive(Debug)]
pub struct WebHandlerParams {
    pub webroot: String,
    pub index: String,
}

#[derive(Debug)]
pub enum Handler {
    File(FileHandlerParams),
    Web(WebHandlerParams),
}

#[derive(Debug)]
pub struct Route {
    pub location: String,
    pub handler: Handler,
}

#[derive(Debug)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug)]
pub struct Server {
    pub listen_ip: String,
    pub port: u16,
    pub log_level: LogLevel,
}

#[derive(Debug)]
pub struct Config {
    pub server: Server,
    pub routes: Vec<Route>,
}

impl Config {
    pub fn from_str(config_str: impl AsRef<str>) -> Result<Self, String> {
        let mut config = Config {
            routes: vec![],
            server: Server {
                listen_ip: "127.0.0.1".into(),
                port: 8000,
                log_level: LogLevel::Info,
            },
        };

        for document in Deserializer::from_str(config_str.as_ref()) {
            let value = Value::deserialize(document).unwrap();

            let server = value
                .get("server")
                .ok_or("missing server section".to_string())?;

            if !server.is_sequence() {
                return Err("malformed server section".to_string());
            }

            for s in server.as_sequence().unwrap() {
                if let Some(listen_ip) = s.get("listen_ip") {
                    config.server.listen_ip = listen_ip.as_str().unwrap_or("".into()).into();
                }

                if let Some(port) = s.get("port") {
                    config.server.port = port.as_u64().unwrap_or(0) as u16;
                }

                if let Some(log_level) = s.get("log_level") {
                    match log_level.as_str().unwrap_or("info") {
                        "debug" => config.server.log_level = LogLevel::Debug,
                        "info" => config.server.log_level = LogLevel::Info,
                        "warn" => config.server.log_level = LogLevel::Warn,
                        "error" => config.server.log_level = LogLevel::Error,
                        _ => config.server.log_level = LogLevel::Info,
                    }
                }
            }

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

                if !location.is_string() {
                    return Err("location should be a string".into());
                }
                if !handler.is_string()
                    || (handler.as_str().unwrap() != "file" && handler.as_str().unwrap() != "web")
                {
                    return Err(format!(
                        "handler not recognized: {}",
                        handler.as_str().unwrap().to_string()
                    ));
                }

                config.routes.push(Route {
                    location: location.as_str().unwrap().to_string(),
                    handler: match handler.as_str().unwrap() {
                        "file" => Handler::File(FileHandlerParams {
                            fs_path: r
                                .get("fs_path")
                                .unwrap_or(&Value::from("."))
                                .as_str()
                                .unwrap_or(".")
                                .to_string(),
                        }),
                        "web" => Handler::Web(WebHandlerParams {
                            webroot: r
                                .get("webroot")
                                .unwrap_or(&Value::from("."))
                                .as_str()
                                .unwrap_or(".")
                                .to_string(),
                            index: r
                                .get("index")
                                .unwrap_or(&Value::from("index.html"))
                                .as_str()
                                .unwrap_or("index.html")
                                .to_string(),
                        }),
                        _ => return Err("bad handler type".into()),
                    },
                })
            }
        }

        Ok(config)
    }
}
