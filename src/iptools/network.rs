use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    net::IpAddr,
};

use crate::iptools::iptrie::BitIp;

/// Trait that defines a generic abstraction for representing network-related operations on IPv4 and IPv6 subnets.
/// This trait is implemented for `Ipv4Network` and `Ipv6Network`.
pub trait ListNetwork: Clone + Debug {
    fn address(&self) -> IpAddr;
    fn bit_network_addr(&self) -> BitIp;
    fn network_prefix(&self) -> u8;
    fn max_prefix(&self) -> u8;
    fn addr_string(&self) -> String;
    fn is_network(&self) -> bool;
    fn is_ipv4(&self) -> bool;
    fn is_ipv6(&self) -> bool;
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    Subnet(T),
    Range(T, T),
}

impl<T> Display for NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.addr_string())
    }
}

impl<T> ListNetwork for NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    fn address(&self) -> IpAddr {
        match self {
            NetworkType::Subnet(net) => net.address(),
            NetworkType::Range(net1, _) => net1.address(),
        }
    }

    fn bit_network_addr(&self) -> BitIp {
        match self {
            NetworkType::Subnet(net) => net.bit_network_addr(),
            NetworkType::Range(net1, _) => net1.bit_network_addr(),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            NetworkType::Subnet(net) => net.network_prefix(),
            NetworkType::Range(_, _) => self.max_prefix(),
        }
    }

    fn max_prefix(&self) -> u8 {
        match self {
            NetworkType::Subnet(net) => net.max_prefix(),
            NetworkType::Range(net1, _) => net1.max_prefix(),
        }
    }

    fn addr_string(&self) -> String {
        match self {
            NetworkType::Subnet(net) => net.addr_string(),
            NetworkType::Range(net1, net2) => {
                format!("{}-{}", net1.address(), net2.address())
            }
        }
    }

    fn is_network(&self) -> bool {
        match self {
            NetworkType::Subnet(net) => net.is_network(),
            NetworkType::Range(net1, net2) => net1.is_network() && net2.is_network(),
        }
    }

    fn is_ipv4(&self) -> bool {
        match self {
            NetworkType::Subnet(net) => net.is_ipv4(),
            NetworkType::Range(net1, net2) => net1.is_ipv4() && net2.is_ipv4(),
        }
    }

    fn is_ipv6(&self) -> bool {
        match self {
            NetworkType::Subnet(net) => net.is_ipv6(),
            NetworkType::Range(net1, net2) => net1.is_ipv6() && net2.is_ipv6(),
        }
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv4 networks (`Ipv4Network`).
impl ListNetwork for Ipv4Network {
    fn address(&self) -> IpAddr {
        IpAddr::V4(self.ip())
    }

    fn bit_network_addr(&self) -> BitIp {
        BitIp::Ipv4(self.network().to_bits())
    }

    fn is_ipv4(&self) -> bool {
        true
    }

    fn is_ipv6(&self) -> bool {
        false
    }

    fn network_prefix(&self) -> u8 {
        self.prefix()
    }

    fn max_prefix(&self) -> u8 {
        32
    }

    fn addr_string(&self) -> String {
        self.to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.ip()
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv6 networks (`Ipv6Network`).
impl ListNetwork for Ipv6Network {
    fn address(&self) -> IpAddr {
        IpAddr::V6(self.ip())
    }

    fn bit_network_addr(&self) -> BitIp {
        BitIp::Ipv6(self.network().to_bits())
    }

    fn is_ipv4(&self) -> bool {
        false
    }

    fn is_ipv6(&self) -> bool {
        true
    }

    fn network_prefix(&self) -> u8 {
        self.prefix()
    }

    fn max_prefix(&self) -> u8 {
        128
    }

    fn addr_string(&self) -> String {
        self.to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.ip()
    }
}

impl ListNetwork for IpAddr {
    fn address(&self) -> IpAddr {
        match self {
            IpAddr::V4(ip) => IpAddr::V4(*ip),
            IpAddr::V6(ip) => IpAddr::V6(*ip),
        }
    }

    fn bit_network_addr(&self) -> BitIp {
        match self {
            IpAddr::V4(ip) => BitIp::Ipv4(ip.to_bits()),
            IpAddr::V6(ip) => BitIp::Ipv6(ip.to_bits()),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        }
    }

    fn max_prefix(&self) -> u8 {
        match self {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        }
    }

    fn addr_string(&self) -> String {
        match self {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => ip.to_string(),
        }
    }

    fn is_network(&self) -> bool {
        match self {
            IpAddr::V4(_) => true,
            IpAddr::V6(_) => true,
        }
    }

    fn is_ipv4(&self) -> bool {
        self.is_ipv4()
    }

    fn is_ipv6(&self) -> bool {
        self.is_ipv6()
    }
}

impl ListNetwork for IpNetwork {
    fn address(&self) -> IpAddr {
        match self {
            IpNetwork::V4(net) => net.address(),
            IpNetwork::V6(net) => net.address(),
        }
    }

    fn bit_network_addr(&self) -> BitIp {
        match self {
            IpNetwork::V4(net) => net.bit_network_addr(),
            IpNetwork::V6(net) => net.bit_network_addr(),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            IpNetwork::V4(net) => net.network_prefix(),
            IpNetwork::V6(net) => net.network_prefix(),
        }
    }

    fn max_prefix(&self) -> u8 {
        match self {
            IpNetwork::V4(net) => net.max_prefix(),
            IpNetwork::V6(net) => net.max_prefix(),
        }
    }

    fn addr_string(&self) -> String {
        match self {
            IpNetwork::V4(net) => net.to_string(),
            IpNetwork::V6(net) => net.to_string(),
        }
    }

    fn is_network(&self) -> bool {
        match self {
            IpNetwork::V4(net) => net.is_network(),
            IpNetwork::V6(net) => net.is_network(),
        }
    }

    fn is_ipv4(&self) -> bool {
        match self {
            IpNetwork::V4(_) => true,
            IpNetwork::V6(_) => false,
        }
    }

    fn is_ipv6(&self) -> bool {
        match self {
            IpNetwork::V4(_) => false,
            IpNetwork::V6(_) => true,
        }
    }
}

impl ListNetwork for IpNet {
    fn address(&self) -> IpAddr {
        match self {
            IpNet::V4(net) => net.address(),
            IpNet::V6(net) => net.address(),
        }
    }

    fn bit_network_addr(&self) -> BitIp {
        match self {
            IpNet::V4(net) => net.bit_network_addr(),
            IpNet::V6(net) => net.bit_network_addr(),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            IpNet::V4(net) => net.network_prefix(),
            IpNet::V6(net) => net.network_prefix(),
        }
    }

    fn max_prefix(&self) -> u8 {
        match self {
            IpNet::V4(net) => net.max_prefix(),
            IpNet::V6(net) => net.max_prefix(),
        }
    }

    fn addr_string(&self) -> String {
        match self {
            IpNet::V4(net) => net.to_string(),
            IpNet::V6(net) => net.to_string(),
        }
    }

    fn is_network(&self) -> bool {
        match self {
            IpNet::V4(net) => net.is_network(),
            IpNet::V6(net) => net.is_network(),
        }
    }

    fn is_ipv4(&self) -> bool {
        match self {
            IpNet::V4(_) => true,
            IpNet::V6(_) => false,
        }
    }

    fn is_ipv6(&self) -> bool {
        match self {
            IpNet::V4(_) => false,
            IpNet::V6(_) => true,
        }
    }
}

impl ListNetwork for Ipv4Net {
    fn address(&self) -> IpAddr {
        IpAddr::V4(self.addr())
    }

    fn bit_network_addr(&self) -> BitIp {
        BitIp::Ipv4(self.network().to_bits())
    }

    fn is_ipv4(&self) -> bool {
        true
    }

    fn is_ipv6(&self) -> bool {
        false
    }

    fn network_prefix(&self) -> u8 {
        self.prefix_len()
    }

    fn max_prefix(&self) -> u8 {
        32
    }

    fn addr_string(&self) -> String {
        self.to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.addr()
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv6 networks (`Ipv6Network`).
impl ListNetwork for Ipv6Net {
    fn address(&self) -> IpAddr {
        IpAddr::V6(self.addr())
    }

    fn bit_network_addr(&self) -> BitIp {
        BitIp::Ipv6(self.network().to_bits())
    }

    fn is_ipv4(&self) -> bool {
        false
    }

    fn is_ipv6(&self) -> bool {
        true
    }

    fn network_prefix(&self) -> u8 {
        self.prefix_len()
    }

    fn max_prefix(&self) -> u8 {
        128
    }

    fn addr_string(&self) -> String {
        self.to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.addr()
    }
}
