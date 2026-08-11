use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlocklistConfig {
    pub url_blocklist: Vec<UrlBlocklist>,
    pub custom_blocklist: CustomListConfig,
    pub custom_allowlist: CustomListConfig,
    pub interval: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomListConfig {
    pub ipv4_folder: String,
    pub ipv6_folder: String,
    #[serde(default)]
    pub split_string: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UrlBlocklist {
    #[serde(default)]
    pub ipv4_url: Option<String>,
    #[serde(default)]
    pub ipv6_url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    #[serde(default = "default_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: std::time::Duration,
    #[serde(default)]
    pub split_string: Option<String>,
    pub backup_path: String,
}
