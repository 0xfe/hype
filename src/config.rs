use std::{error, fmt};

use serde::Deserialize;
use serde_yaml::{Deserializer, Value};

#[derive(Debug)]
pub struct FileHandlerParams {
    pub fs_path: String,
    pub trailing_slashes: bool,
}

#[derive(Debug)]
pub struct WebHandlerParams {
    pub webroot: String,
    pub index: String,
    pub hosts: Vec<String>,
    pub trailing_slashes: bool,
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

#[derive(Debug, Clone)]
pub enum ConfigError {
    MissingField(String),
    MalformedField(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let e = match self {
            Self::MissingField(field) => format!("missing field: {}", field),
            Self::MalformedField(field) => format!("malformed field: {}", field),
        };

        write!(f, "ConfigError: {}", e)
    }
}

impl error::Error for ConfigError {}

impl Config {
    pub fn from(config_str: impl AsRef<str>) -> Result<Self, ConfigError> {
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

            let server_seq = value
                .get("server")
                .ok_or(ConfigError::MissingField("server".into()))?
                .as_sequence()
                .ok_or(ConfigError::MalformedField("server".into()))?;

            for s in server_seq {
                config.server.listen_ip = s
                    .get("listen_ip")
                    .ok_or(ConfigError::MissingField("listen_ip".to_string()))?
                    .as_str()
                    .ok_or(ConfigError::MalformedField("listen_ip".to_string()))?
                    .into();

                config.server.port = s
                    .get("port")
                    .ok_or(ConfigError::MissingField("port".to_string()))?
                    .as_u64()
                    .ok_or(ConfigError::MalformedField("port".to_string()))?
                    as u16;

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

            let routes_seq = value
                .get("routes")
                .ok_or(ConfigError::MissingField("routes".into()))?
                .as_sequence()
                .ok_or(ConfigError::MalformedField("routes".into()))?;

            for r in routes_seq {
                let location = r
                    .get("location")
                    .ok_or(ConfigError::MissingField("route:location".into()))?
                    .as_str()
                    .ok_or(ConfigError::MalformedField("route:location".to_string()))?
                    .to_string();

                let handler = r
                    .get("handler")
                    .ok_or(ConfigError::MissingField("route:handler".into()))?
                    .as_str()
                    .ok_or(ConfigError::MalformedField("route:handler".to_string()))?
                    .to_string();

                config.routes.push(Route {
                    location,
                    handler: match handler.as_str() {
                        "file" => Handler::File(FileHandlerParams {
                            fs_path: r
                                .get("fs_path")
                                .unwrap_or(&Value::from("."))
                                .as_str()
                                .unwrap_or(".")
                                .to_string(),
                            trailing_slashes: r
                                .get("trailing_slashes")
                                .unwrap_or(&Value::from(true))
                                .as_bool()
                                .unwrap_or(true),
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
                            hosts: r
                                .get("hosts")
                                .unwrap_or(&Value::from(Vec::<Value>::new()))
                                .as_sequence()
                                .unwrap_or(&vec![])
                                .iter()
                                .map(|v| v.as_str().unwrap_or("").to_string())
                                .collect(),
                            trailing_slashes: r
                                .get("trailing_slashes")
                                .unwrap_or(&Value::from(true))
                                .as_bool()
                                .unwrap_or(true),
                        }),
                        _ => {
                            return Err(ConfigError::MalformedField(format!(
                                "route:handler: {}",
                                handler
                            )))
                        }
                    },
                })
            }
        }

        Ok(config)
    }
}
