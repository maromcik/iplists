use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    net::IpAddr,
};

/// Trait that defines a generic abstraction for representing network-related operations on IPv4 and IPv6 subnets.
/// This trait is implemented for `Ipv4Network` and `Ipv6Network`.
pub trait ListNetwork: Clone + Debug {
    fn addr(&self) -> IpAddr;
    fn network_prefix(&self) -> u8;
    fn max_prefix(&self) -> u8;
    fn network_string(&self) -> String;
    fn is_network(&self) -> bool;
    fn is_ipv4(&self) -> bool;
    fn is_ipv6(&self) -> bool;
    fn from_ip_addr(ip: IpAddr) -> Option<Self>;
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    Ip(T),
    Range(T, T),
}

impl<T> Display for NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.network_string())
    }
}

impl<T> ListNetwork for NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    fn addr(&self) -> IpAddr {
        match self {
            NetworkType::Ip(net) => net.addr(),
            NetworkType::Range(net1, _) => net1.addr(),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            NetworkType::Ip(net) => net.network_prefix(),
            NetworkType::Range(_, _) => self.max_prefix(),
        }
    }

    fn max_prefix(&self) -> u8 {
        match self {
            NetworkType::Ip(net) => net.max_prefix(),
            NetworkType::Range(net1, _) => net1.max_prefix(),
        }
    }

    fn network_string(&self) -> String {
        match self {
            NetworkType::Ip(net) => net.network_string(),
            NetworkType::Range(net1, net2) => {
                format!("{}-{}", net1.addr(), net2.addr())
            }
        }
    }

    fn is_network(&self) -> bool {
        match self {
            NetworkType::Ip(net) => net.is_network(),
            NetworkType::Range(net1, net2) => net1.is_network() && net2.is_network(),
        }
    }

    fn is_ipv4(&self) -> bool {
        match self {
            NetworkType::Ip(net) => net.is_ipv4(),
            NetworkType::Range(net1, net2) => net1.is_ipv4() && net2.is_ipv4(),
        }
    }

    fn is_ipv6(&self) -> bool {
        match self {
            NetworkType::Ip(net) => net.is_ipv6(),
            NetworkType::Range(net1, net2) => net1.is_ipv6() && net2.is_ipv6(),
        }
    }

    fn from_ip_addr(ip: IpAddr) -> Option<Self> {
        Some(NetworkType::Ip(T::from_ip_addr(ip)?))
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv4 networks (`Ipv4Network`).
impl ListNetwork for Ipv4Network {
    fn addr(&self) -> IpAddr {
        IpAddr::V4(self.ip())
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

    fn network_string(&self) -> String {
        self.network().to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.ip()
    }

    fn from_ip_addr(ip: IpAddr) -> Option<Self> {
        match ip {
            IpAddr::V4(ip) => Ipv4Network::new(ip, 32).ok(),
            IpAddr::V6(_) => None,
        }
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv6 networks (`Ipv6Network`).
impl ListNetwork for Ipv6Network {
    fn addr(&self) -> IpAddr {
        IpAddr::V6(self.ip())
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

    fn network_string(&self) -> String {
        self.network().to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.ip()
    }

    fn from_ip_addr(ip: IpAddr) -> Option<Self> {
        match ip {
            IpAddr::V4(_) => None,
            IpAddr::V6(ip) => Ipv6Network::new(ip, 128).ok(),
        }
    }
}

impl ListNetwork for IpAddr {
    fn addr(&self) -> IpAddr {
        match self {
            IpAddr::V4(ip) => IpAddr::V4(*ip),
            IpAddr::V6(ip) => IpAddr::V6(*ip),
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

    fn network_string(&self) -> String {
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
        match self {
            IpAddr::V4(_) => true,
            IpAddr::V6(_) => false,
        }
    }

    fn is_ipv6(&self) -> bool {
        match self {
            IpAddr::V4(_) => false,
            IpAddr::V6(_) => true,
        }
    }

    fn from_ip_addr(ip: IpAddr) -> Option<Self> {
        Some(ip)
    }
}

impl ListNetwork for IpNetwork {
    fn addr(&self) -> IpAddr {
        match self {
            IpNetwork::V4(net) => net.addr(),
            IpNetwork::V6(net) => net.addr(),
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

    fn network_string(&self) -> String {
        match self {
            IpNetwork::V4(net) => net.network_string(),
            IpNetwork::V6(net) => net.network_string(),
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

    fn from_ip_addr(ip: IpAddr) -> Option<Self> {
        Some(match ip {
            IpAddr::V4(ip) => IpNetwork::V4(Ipv4Network::new(ip, 32).ok()?),
            IpAddr::V6(ip) => IpNetwork::V6(Ipv6Network::new(ip, 128).ok()?),
        })
    }
}
