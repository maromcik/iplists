use std::net::IpAddr;

use serde::Deserialize;

use crate::{forms::IpVersion, iplist::formatter::OutputFormat};

/// Query parameters of `GET /api/iplist/location`.
///
/// `country` (ISO alpha-2) and `continent` are mutually exclusive; with
/// both absent all known networks are returned. `format` defaults to JSON.
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

/// Query parameters of `GET /api/iplist/asn`.
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct IpListFormByAsn {
    #[serde(default)]
    pub asn: Option<u32>,
    #[serde(default)]
    pub format: OutputFormat,
    #[serde(default)]
    pub version: Option<IpVersion>,
}

/// Query parameters of `GET /api/iplist/geo`.
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub struct ApiGeoLocation {
    pub ip: IpAddr,
}
