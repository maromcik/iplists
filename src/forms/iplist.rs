use serde::Deserialize;

use crate::{forms::IpVersion, iplist::formatter::OutputFormat};

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct IpListFormByCountry {
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub continent: Option<String>,
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub version: Option<IpVersion>,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct IpListFormByAsn {
    #[serde(default)]
    pub asn: Option<u32>,
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub version: Option<IpVersion>,
}
