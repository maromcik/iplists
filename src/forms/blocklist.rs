use serde::Deserialize;

use crate::forms::IpVersion;

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct BlocklistIpVersion {
    #[serde(default)]
    pub version: Option<IpVersion>,
}
