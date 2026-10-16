use serde::Deserialize;

use crate::{forms::IpVersion, iplist::formatter::OutputFormat};

/// Query parameters of `GET /api/blocklist`.
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct BlocklistIpVersion {
    #[serde(default)]
    pub version: Option<IpVersion>,
    #[serde(default)]
    pub format: OutputFormat,
}
