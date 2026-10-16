use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

fn default_max_age() -> std::time::Duration {
    std::time::Duration::from_hours(24)
}

/// Optional HTTP basic-auth credentials for the download endpoints.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct BasicAuth {
    pub username: String,
    pub password: String,
}

/// Configuration of the geo-IP / ASN data pipeline (`iplist:` in the
/// config file).
///
/// Both URI templates support `{year}`/`{month}`/`{day}` placeholders that
/// are expanded at download time (see [`crate::iplist::fetch::Downloader`]).
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct IplistConfig {
    /// Download URL for the country/locations archive.
    pub country_uri: String,
    /// Download URL for the ASN archive.
    pub asn_uri: String,
    /// HTTP timeout for downloads (default 10s).
    #[serde(default = "default_timeout")]
    #[serde(with = "humantime_serde")]
    pub timeout: std::time::Duration,
    /// Maximum age of the cached archive before re-downloading (default 24h).
    #[serde(default = "default_max_age")]
    #[serde(with = "humantime_serde")]
    pub max_age: std::time::Duration,
    /// Extra HTTP headers for the download requests (e.g. API keys).
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(default)]
    pub basic_auth: Option<BasicAuth>,
    /// Root folder for downloaded archives and generated list files.
    pub output_folder: String,
    /// Cron expression (6 fields, seconds required) for the refresh job.
    pub cron: String,
}
