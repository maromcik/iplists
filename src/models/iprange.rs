use ipnet::IpNet;
use serde::Serialize;

use crate::iplist::parse::Location;

#[derive(Serialize, Debug, Clone)]
pub struct CombinedIpRange {
    pub network: IpNet,
    pub asn: u32,
    pub isp: String,
    pub location: Location,
}

impl CombinedIpRange {
    pub fn new(network: IpNet, asn: u32, isp: String, location: Location) -> Self {
        Self {
            network,
            asn,
            isp,
            location,
        }
    }
}
