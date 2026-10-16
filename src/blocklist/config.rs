use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

/// Configuration of the blocklist pipeline (`blocklist:` in the config file).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BlocklistConfig {
    /// Remote lists fetched over HTTP(S) on every refresh.
    pub url_blocklist: Vec<UrlBlocklist>,
    /// Extra local networks to always block, loaded from disk.
    pub custom_blocklist: CustomListConfig,
    /// Local networks subtracted from the merged blocklist (allowlist).
    pub custom_allowlist: CustomListConfig,
    /// Cron expression (6 fields, seconds required) for the refresh job.
    pub cron: String,
}

/// Local list folders for the custom block/allow lists. Every file inside
/// the folders is parsed as a list of CIDRs.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CustomListConfig {
    pub ipv4_folder: String,
    pub ipv6_folder: String,
    /// Optional delimiter between entries; defaults to any whitespace.
    #[serde(default)]
    pub split_string: Option<String>,
}

/// One remote blocklist source.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UrlBlocklist {
    /// Human-readable name used in logs and status messages.
    pub name: String,
    #[serde(default)]
    pub ipv4_url: Option<String>,
    #[serde(default)]
    pub ipv6_url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    #[serde(default = "default_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: std::time::Duration,
    /// Optional delimiter between entries; defaults to any whitespace.
    #[serde(default)]
    pub split_string: Option<String>,
    /// Where a JSON copy of the last successfully downloaded list is kept,
    /// used as a fallback when fetching fails.
    pub backup_path: String,
    /// Treat a parsed-but-empty remote list as a hard error (keeps the
    /// previous blocklist in place instead of clearing it).
    #[serde(default)]
    pub error_on_empty: bool,
}
