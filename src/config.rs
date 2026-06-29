use std::collections::HashSet;

use config::Config;
use serde::{Deserialize, Serialize};

use crate::{error::AppError, iplist::config::GeoConfig};

fn default_info() -> String {
    String::from("info")
}

fn default_true() -> Option<bool> {
    Some(true)
}

fn default_static_files() -> String {
    String::from("static")
}

fn default_web_hostname() -> HashSet<String> {
    HashSet::from(["[::]:8000".into()])
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppConfig {
    #[serde(default = "default_web_hostname")]
    pub hostnames: HashSet<String>,
    pub geo: GeoConfig,
    #[serde(default = "default_info")]
    pub app_log_level: String,
    #[serde(default = "default_info")]
    pub all_log_level: String,
    #[serde(default = "default_static_files")]
    pub static_files_path: String,
}

impl AppConfig {
    pub fn parse_config(settings_path: &str) -> Result<AppConfig, AppError> {
        let settings = Config::builder()
            .add_source(config::File::with_name(settings_path))
            .add_source(config::Environment::with_prefix("APP"))
            .build()?;

        let config = settings.try_deserialize::<AppConfig>()?;

        Ok(config)
    }
}
