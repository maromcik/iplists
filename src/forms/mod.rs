use serde::Deserialize;

pub mod blocklist;
pub mod extractors;
pub mod iplist;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    Ipv4,
    Ipv6,
}
