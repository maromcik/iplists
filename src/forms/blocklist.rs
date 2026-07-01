use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    Ipv4,
    Ipv6,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub struct BlocklistIpVersion {
    #[serde(default)]
    pub version: Option<IpVersion>,
}
