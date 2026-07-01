use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlocklistConfig {
    pub ipv4_url: String,
    pub ipv6_url: String,
    pub headers: Option<HashMap<String, String>>,
    pub split_string: Option<String>,
    pub timeout: u64,
    pub download_cron: String,
}
