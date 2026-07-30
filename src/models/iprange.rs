use std::net::IpAddr;

use ipnet::IpNet;
use serde::Serialize;

use crate::iplist::iprange::Location;

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
