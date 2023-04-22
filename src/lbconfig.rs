use std::{error, fmt};

use rand::{distributions::Alphanumeric, Rng};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect::<String>()
}

fn default_tls_cert_file() -> String {
    String::from("localhost.crt")
}

fn default_tls_key_file() -> String {
    String::from("localhost.key")
}

fn random_default(prefix: &str) -> String {
    format!("{}-{}", prefix, random_string(7))
}

macro_rules! ConfigID {
    ($struct_name: ident, $prefix:expr) => {
        #[derive(Debug, Clone, Deserialize, Eq, PartialEq, Hash)]
        pub struct $struct_name(pub String);

        impl Default for $struct_name {
            fn default() -> Self {
                $struct_name(random_default($prefix.to_lowercase().as_str()))
            }
        }

        impl From<$struct_name> for String {
            fn from(id: $struct_name) -> Self {
                id.0
            }
        }

        impl AsRef<str> for $struct_name {
            fn as_ref(&self) -> &str {
                self.0.as_str()
            }
        }

        impl AsRef<String> for $struct_name {
            fn as_ref(&self) -> &String {
                &self.0
            }
        }

        impl AsRef<$struct_name> for String {
            fn as_ref(&self) -> &$struct_name {
                unsafe { &*(self as *const String as *const $struct_name) }
            }
        }
    };
}

ConfigID!(BackendId, "be");
ConfigID!(RouteId, "rt");

#[derive(Debug, Deserialize)]
pub struct Server {
    pub listen_ip: String,
    pub port: u16,
    pub log_level: LogLevel,

    #[serde(default)]
    pub enable_tls: bool,

    #[serde(default = "default_tls_cert_file")]
    pub tls_cert_file: String,
    #[serde(default = "default_tls_key_file")]
    pub tls_key_file: String,
}

impl Default for Server {
    fn default() -> Self {
        Self {
            listen_ip: String::from("localhost"),
            port: 8000,
            log_level: LogLevel::Info,
            enable_tls: false,
            tls_cert_file: default_tls_cert_file(),
            tls_key_file: default_tls_key_file(),
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Backend {
    #[serde(default)]
    pub id: BackendId,
    pub host: String,
    pub port: u16,

    #[serde(default)]
    pub enable_tls: bool,

    #[serde(default)]
    pub weight: u32,
}

#[derive(Debug, Deserialize)]
pub struct Route {
    #[serde(default)]
    pub id: RouteId,
    pub location: String,
    pub host_header: Option<String>,
    pub backends: Vec<Backend>,
}

#[derive(Debug, Deserialize, Default)]
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
    pub fn from(config_str: impl AsRef<str>) -> Result<Self, ConfigError> {
        let config =
            serde_yaml::from_str(config_str.as_ref()).map_err(|e| ConfigError(e.to_string()))?;
        Ok(config)
    }
}
