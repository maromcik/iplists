use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use std::{
    fmt::{Debug, Display},
    net::IpAddr,
    str::FromStr,
};

use crate::{
    iplist::iprange::{IpAsnRange, IpLocationRange},
    iptools::iptrie::{BitIp, TrieKey},
};

/// Trait that defines a generic abstraction for representing network-related operations on IPv4 and IPv6 subnets.
/// This trait is implemented for `Ipv4Network` and `Ipv6Network`.
pub trait ListNetwork: Clone + Debug {
    fn address(&self) -> IpAddr;
    fn network_prefix(&self) -> u8;
    fn trie_key(&self) -> TrieKey;
    fn network_string(&self) -> String;
    fn is_net(&self) -> bool;
    fn is_ipv4(&self) -> bool;
    fn is_ipv6(&self) -> bool;
}

impl ListNetwork for IpAddr {
    fn address(&self) -> IpAddr {
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

    fn trie_key(&self) -> TrieKey {
        match self {
            IpAddr::V4(ip) => TrieKey::new(BitIp::Ipv4(ip.to_bits()), 32),
            IpAddr::V6(ip) => TrieKey::new(BitIp::Ipv6(ip.to_bits()), 128),
        }
    }

    fn network_string(&self) -> String {
        match self {
            IpAddr::V4(ip) => ip.to_string(),
            IpAddr::V6(ip) => ip.to_string(),
        }
    }

    fn is_net(&self) -> bool {
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

impl ListNetwork for IpNet {
    fn address(&self) -> IpAddr {
        match self {
            IpNet::V4(net) => net.address(),
            IpNet::V6(net) => net.address(),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            IpNet::V4(net) => net.network_prefix(),
            IpNet::V6(net) => net.network_prefix(),
        }
    }

    fn trie_key(&self) -> TrieKey {
        match self {
            IpNet::V4(net) => net.trie_key(),
            IpNet::V6(net) => net.trie_key(),
        }
    }

    fn network_string(&self) -> String {
        match self {
            IpNet::V4(net) => net.to_string(),
            IpNet::V6(net) => net.to_string(),
        }
    }

    fn is_net(&self) -> bool {
        match self {
            IpNet::V4(net) => net.is_net(),
            IpNet::V6(net) => net.is_net(),
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

    fn trie_key(&self) -> TrieKey {
        TrieKey::new(BitIp::Ipv4(self.network().to_bits()), self.network_prefix())
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

    fn network_string(&self) -> String {
        self.to_string()
    }

    fn is_net(&self) -> bool {
        self.network() == self.addr()
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv6 networks (`Ipv6Network`).
impl ListNetwork for Ipv6Net {
    fn address(&self) -> IpAddr {
        IpAddr::V6(self.addr())
    }

    fn trie_key(&self) -> TrieKey {
        TrieKey::new(BitIp::Ipv6(self.network().to_bits()), self.network_prefix())
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

    fn network_string(&self) -> String {
        self.to_string()
    }

    fn is_net(&self) -> bool {
        self.network() == self.addr()
    }
}

impl ListNetwork for IpAsnRange {
    fn address(&self) -> std::net::IpAddr {
        self.network.address()
    }

    fn trie_key(&self) -> TrieKey {
        self.network.trie_key()
    }

    fn network_prefix(&self) -> u8 {
        self.network.network_prefix()
    }

    fn network_string(&self) -> String {
        self.network.network_string()
    }

    fn is_net(&self) -> bool {
        self.network.is_net()
    }

    fn is_ipv4(&self) -> bool {
        self.network.is_ipv4()
    }

    fn is_ipv6(&self) -> bool {
        self.network.is_ipv6()
    }
}

impl ListNetwork for IpLocationRange {
    fn address(&self) -> std::net::IpAddr {
        self.network.address()
    }

    fn trie_key(&self) -> TrieKey {
        self.network.trie_key()
    }

    fn network_prefix(&self) -> u8 {
        self.network.network_prefix()
    }

    fn network_string(&self) -> String {
        self.network.network_string()
    }

    fn is_net(&self) -> bool {
        self.network.is_net()
    }

    fn is_ipv4(&self) -> bool {
        self.network.is_ipv4()
    }

    fn is_ipv6(&self) -> bool {
        self.network.is_ipv6()
    }
}

impl ListNetwork for Ipv4Network {
    fn address(&self) -> IpAddr {
        IpAddr::V4(self.ip())
    }

    fn trie_key(&self) -> TrieKey {
        TrieKey::new(BitIp::Ipv4(self.network().to_bits()), self.network_prefix())
    }

    fn network_prefix(&self) -> u8 {
        self.prefix()
    }

    fn network_string(&self) -> String {
        self.to_string()
    }

    fn is_net(&self) -> bool {
        self.network() == self.ip()
    }

    fn is_ipv4(&self) -> bool {
        true
    }

    fn is_ipv6(&self) -> bool {
        false
    }
}

impl ListNetwork for Ipv6Network {
    fn address(&self) -> IpAddr {
        IpAddr::V6(self.ip())
    }

    fn trie_key(&self) -> TrieKey {
        TrieKey::new(BitIp::Ipv6(self.network().to_bits()), self.network_prefix())
    }

    fn network_prefix(&self) -> u8 {
        self.prefix()
    }

    fn network_string(&self) -> String {
        self.to_string()
    }

    fn is_net(&self) -> bool {
        self.network() == self.ip()
    }

    fn is_ipv4(&self) -> bool {
        false
    }

    fn is_ipv6(&self) -> bool {
        true
    }
}

impl ListNetwork for IpNetwork {
    fn address(&self) -> IpAddr {
        match self {
            IpNetwork::V4(net) => IpAddr::V4(net.ip()),
            IpNetwork::V6(net) => IpAddr::V6(net.ip()),
        }
    }

    fn trie_key(&self) -> TrieKey {
        match self {
            IpNetwork::V4(net) => TrieKey::new(BitIp::Ipv4(net.network().to_bits()), net.prefix()),
            IpNetwork::V6(net) => TrieKey::new(BitIp::Ipv6(net.network().to_bits()), net.prefix()),
        }
    }

    fn network_prefix(&self) -> u8 {
        self.prefix()
    }

    fn network_string(&self) -> String {
        self.to_string()
    }

    fn is_net(&self) -> bool {
        self.network() == self.ip()
    }

    fn is_ipv4(&self) -> bool {
        matches!(self, IpNetwork::V4(_))
    }

    fn is_ipv6(&self) -> bool {
        matches!(self, IpNetwork::V6(_))
    }
}

#[allow(dead_code)]
pub fn summarize_ranges<T>(start: T, end: T) -> Vec<T>
where
    T: ListNetwork + FromStr + Display + std::fmt::Debug + From<IpNet>,
{
    match (start.address(), end.address()) {
        (IpAddr::V4(ipv4_addr1), IpAddr::V4(ipv4_addr2)) => {
            ipnet::IpSubnets::V4(ipnet::Ipv4Subnets::new(ipv4_addr1, ipv4_addr2, 0))
                .into_iter()
                .map(|net| T::from(net))
                .collect()
        }
        (IpAddr::V6(ipv6_addr1), IpAddr::V6(ipv6_addr2)) => {
            ipnet::IpSubnets::V6(ipnet::Ipv6Subnets::new(ipv6_addr1, ipv6_addr2, 0))
                .into_iter()
                .map(|net| T::from(net))
                .collect()
        }
        _ => vec![],
    }
}
