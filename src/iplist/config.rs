use std::collections::HashMap;

use serde::{Deserialize, Serialize};

fn default_timeout() -> std::time::Duration {
    std::time::Duration::from_secs(10)
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct GeoConfig {
    pub country_uri: String,
    pub asn_uri: String,
    #[serde(default = "default_timeout")]
    pub timeout: std::time::Duration,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub location_path: String,
    pub output_folder: String,
}
