use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    net::IpAddr,
};

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
        match self {
            NetworkType::Ip(net) => write!(f, "{}", net.network_string()),
            NetworkType::Range(net1, net2) => {
                write!(f, "{}-{}", net1.network_string(), net2.network_string())
            }
        }
    }
}

impl<T> ListNetwork for NetworkType<T>
where
    T: ListNetwork + Clone + Debug,
{
    fn network_addr(&self) -> BitIp {
        match self {
            NetworkType::Ip(net) => net.network_addr(),
            NetworkType::Range(net1, _) => net1.network_addr(),
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
                format!("{}-{}", net1.network_string(), net2.network_string())
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

/// Trait that defines a generic abstraction for representing network-related operations on IPv4 and IPv6 subnets.
/// This trait is implemented for `Ipv4Network` and `Ipv6Network`.
pub trait ListNetwork: Clone + Debug {
    /// Retrieves the numeric representation (as `BitIp`) of the network address.
    ///
    /// # Returns
    /// A `BitIp` containing the numeric representation of the network address.
    fn network_addr(&self) -> BitIp;

    /// Retrieves the network prefix length, which is the number of bits used for the network part of the address.
    ///
    /// # Returns
    /// An unsigned 8-bit integer (`u8`) representing the prefix length.
    fn network_prefix(&self) -> u8;

    /// Provides the maximum allowable prefix length for the network type.
    ///
    /// # Returns
    /// - `32` for IPv4 networks.
    /// - `128` for IPv6 networks.
    fn max_prefix(&self) -> u8;

    /// Converts the network address to its string representation (e.g., `192.168.0.0/24`).
    ///
    /// # Returns
    /// A `String` containing the CIDR representation of the network.
    fn network_string(&self) -> String;

    /// Checks whether the current network is properly aligned to the prefix boundary.
    ///
    /// # Returns
    /// `true` if the network address is aligned; otherwise, `false`.
    fn is_network(&self) -> bool;

    fn is_ipv4(&self) -> bool;
    fn is_ipv6(&self) -> bool;

    /// Creates a network value from a single IP address.
    ///
    /// For address-family-specific types (`Ipv4Network`, `Ipv6Network`),
    /// returns `None` if the address family does not match.
    fn from_ip_addr(ip: IpAddr) -> Option<Self>;
}

/// Implementation of the `BlockListNetwork` trait for IPv4 networks (`Ipv4Network`).
impl ListNetwork for Ipv4Network {
    fn network_addr(&self) -> BitIp {
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
    fn network_addr(&self) -> BitIp {
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
    fn network_addr(&self) -> BitIp {
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
    fn network_addr(&self) -> BitIp {
        match self {
            IpNetwork::V4(net) => net.network_addr(),
            IpNetwork::V6(net) => net.network_addr(),
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

/// Represents a generic IP address in either IPv4 or IPv6 format using numeric representations.
pub enum BitIp {
    Ipv4(u32),
    Ipv6(u128),
}

impl BitIp {
    /// Performs a right-shift operation on an IP address by `n` bits
    /// and returns the result in the corresponding `BitIp` format.
    ///
    /// # Parameters
    /// - `n`: The number of bits to shift.
    ///
    /// # Returns
    /// The shifted `BitIp` instance.
    pub(crate) fn r_shift(&self, n: u8) -> Self {
        match self {
            BitIp::Ipv4(ip) => BitIp::Ipv4(*ip >> n),
            BitIp::Ipv6(ip) => BitIp::Ipv6(*ip >> n),
        }
    }

    /// Performs a bitwise AND operation between the IP address and the given `rhs` value.
    ///
    /// # Parameters
    /// - `rhs`: The value to AND with (8 bits for this implementation).
    ///
    /// # Returns
    /// The result of the operation as an 8-bit value.
    pub(crate) fn b_and(self, rhs: u8) -> u8 {
        match self {
            BitIp::Ipv4(ip) => (ip & rhs as u32) as u8,
            BitIp::Ipv6(ip) => (ip & rhs as u128) as u8,
        }
    }
}
