use serde::Deserialize;

use crate::iplist::formatter::OutputFormat;

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct IpListFormByCountry {
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub continent: Option<String>,
    #[serde(default)]
    pub format: OutputFormat,
}

#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct IpListFormByAsn {
    #[serde(default)]
    pub asn: Option<u32>,
    #[serde(default)]
    pub format: OutputFormat,
}
