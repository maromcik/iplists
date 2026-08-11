use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

fn default_max_age() -> std::time::Duration {
    std::time::Duration::from_hours(24)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IplistConfig {
    pub country_uri: String,
    pub asn_uri: String,
    #[serde(default = "default_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: std::time::Duration,
    #[serde(default = "default_max_age")]
    #[serde(with = "humantime_serde")]
    pub max_age: std::time::Duration,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub basic_auth: Option<BasicAuth>,
    pub output_folder: String,
    pub cron: String,
}
