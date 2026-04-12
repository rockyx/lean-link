use std::time::Duration;

use serde::{Deserialize, Serialize};

pub mod middleware;
pub mod service;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct WebConfig {
    pub host: String,
    pub port: u16,
}

impl Default for WebConfig {
    fn default() -> Self {
        WebConfig {
            host: "127.0.0.1".to_string(),
            port: 8080,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JwtConfig {
    pub secret: String,
    #[serde(with = "crate::utils::datetime::string_to_duration")]
    pub expires_in: Duration,
}

impl Default for JwtConfig {
    fn default() -> Self {
        JwtConfig {
            secret: "secret".to_string(),
            expires_in: Duration::from_secs(3600),
        }
    }
}