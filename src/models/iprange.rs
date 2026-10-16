use std::net::IpAddr;

use ipnet::IpNet;
use serde::Serialize;

use crate::iplist::iprange::Location;

/// JSON response of `GET /api/iplist/geo`: the queried address plus the
/// containing network, its ASN/ISP and its geographic location.
/// Missing pieces are zero-valued (e.g. unknown ASN is `0`, unknown
/// location has empty strings).
#[derive(Serialize, Debug, Clone)]
pub struct CombinedIpRange {
    pub ip: IpAddr,
    pub network: IpNet,
    pub asn: u32,
    pub isp: String,
    pub location: Location,
}

impl CombinedIpRange {
    pub fn new(ip: IpAddr, network: IpNet, asn: u32, isp: String, location: Location) -> Self {
        Self {
            ip,
            network,
            asn,
            isp,
            location,
        }
    }
}
