use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

fn default_max_age() -> std::time::Duration {
    std::time::Duration::from_hours(24)
}

fn default_split_ranges() -> bool {
    true
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
    pub location_path: String,
    pub output_folder: String,
    pub download_cron: String,
    #[serde(default = "default_split_ranges")]
    pub split_ranges: bool,
}
