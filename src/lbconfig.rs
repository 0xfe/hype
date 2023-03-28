use std::{error, fmt};

use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub listen_ip: String,
    pub port: u16,
    pub log_level: LogLevel,

    #[serde(default)]
    pub enable_tls: bool,
    pub tls_cert_file: Option<String>,
    pub tls_key_file: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Backend {
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub enable_tls: bool,

    #[serde(default)]
    pub weight: u32,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    pub location: String,
    pub host_header: Option<String>,
    pub backends: Vec<Backend>,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(flatten)]
    pub server: Server,
    pub routes: Vec<Route>,
}

#[derive(Debug, Clone)]
pub struct ConfigError(String);

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "lbconfig error: {}", self.0)
    }
}

impl error::Error for ConfigError {}

impl Config {
    pub fn from_str(config_str: impl AsRef<str>) -> Result<Self, ConfigError> {
        let config =
            serde_yaml::from_str(config_str.as_ref()).map_err(|e| ConfigError(e.to_string()))?;
        Ok(config)
    }
}
