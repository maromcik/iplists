use std::collections::HashSet;

use config::Config;
use serde::{Deserialize, Serialize};

use crate::blocklist::config::BlocklistConfig;
use crate::{error::AppError, iplist::config::IplistConfig};

fn default_info() -> String {
    String::from("info")
}

fn default_static_files() -> String {
    String::from("static")
}

fn default_web_hostname() -> HashSet<String> {
    HashSet::from(["[::]:8000".into()])
}

/// Root of the application configuration.
///
/// Deserialized from a YAML/TOML file (default `iplists.yaml`) with
/// environment-variable overrides using the `APP` prefix (see
/// [`AppConfig::parse_config`]). See `iplists-example.yaml` for a fully
/// documented example.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    /// `<host>:<port>` or `<addr>:<port>` values to bind listeners to.
    /// Resolved through DNS, so multiple addresses may be bound per entry.
    #[serde(default = "default_web_hostname")]
    pub hostnames: HashSet<String>,
    /// Geo-IP / ASN list download and refresh settings.
    pub iplist: IplistConfig,
    /// Blocklist sources, custom lists and refresh schedule.
    pub blocklist: BlocklistConfig,
    #[serde(default = "default_info")]
    pub app_log_level: String,
    #[serde(default = "default_info")]
    pub all_log_level: String,
    #[serde(default = "default_static_files")]
    pub static_files_path: String,
    /// Number of tokio worker threads; defaults to the CPU core count.
    #[serde(default)]
    pub workers: Option<usize>,
    /// PEM certificate/chain and private key; TLS is enabled only when both are set.
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

impl AppConfig {
    /// Loads the configuration file (YAML or TOML, extension inferred) and
    /// layers `APP_*` environment variables on top of it.
    pub fn parse_config(settings_path: &str) -> Result<AppConfig, AppError> {
        let settings = Config::builder()
            .add_source(config::File::with_name(settings_path))
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;

        let config = settings.try_deserialize::<AppConfig>()?;

        Ok(config)
    }
}
