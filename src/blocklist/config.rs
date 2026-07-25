use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlocklistConfig {
    pub ipv4_url: String,
    pub ipv6_url: String,
    pub ipv4_folder: String,
    pub ipv6_folder: String,
    pub headers: Option<HashMap<String, String>>,
    pub split_string: Option<String>,
    #[serde(default = "default_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: std::time::Duration,
    pub download_cron: String,
}
