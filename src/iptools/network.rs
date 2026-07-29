use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use serde::{Deserialize, Serialize};
use std::{
    cmp::max,
    fmt::{Debug, Display},
    net::IpAddr,
};

use crate::{
    iplist::iprange::{IpAsnRange, IpLocationRange},
    iptools::iptrie::{BitIp, TrieKey},
};

/// Trait that defines a generic abstraction for representing network-related operations on IPv4 and IPv6 subnets.
/// This trait is implemented for `Ipv4Network` and `Ipv6Network`.
pub trait ListNetwork: Clone + Debug {
    fn network(&self) -> IpNet;
    fn address(&self) -> IpAddr;
    fn network_prefix(&self) -> u8;
    fn trie_key(&self) -> Option<TrieKey>;
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
    fn network(&self) -> IpNet {
        match self {
            NetworkType::Subnet(net) => net.network(),
            NetworkType::Range(net1, _) => net1.network(),
        }
    }

    fn address(&self) -> IpAddr {
        match self {
            NetworkType::Subnet(net) => net.address(),
            NetworkType::Range(net1, _) => net1.address(),
        }
    }

    fn network_prefix(&self) -> u8 {
        match self {
            NetworkType::Subnet(net) => net.network_prefix(),
            NetworkType::Range(net1, net2) => max(net1.network_prefix(), net2.network_prefix()),
        }
    }

    fn trie_key(&self) -> Option<TrieKey> {
        match self {
            NetworkType::Subnet(net) => net.trie_key(),
            NetworkType::Range(_, _) => None,
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
impl ListNetwork for IpAddr {
    fn network(&self) -> IpNet {
        let prefix = match self {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        IpNet::new(*self, prefix).expect("Invalid network")
    }

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

    fn trie_key(&self) -> Option<TrieKey> {
        match self {
            IpAddr::V4(ip) => Some(TrieKey::new(BitIp::Ipv4(ip.to_bits()), 32)),
            IpAddr::V6(ip) => Some(TrieKey::new(BitIp::Ipv6(ip.to_bits()), 128)),
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

impl ListNetwork for IpNet {
    fn network(&self) -> IpNet {
        *self
    }

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

    fn trie_key(&self) -> Option<TrieKey> {
        match self {
            IpNet::V4(net) => net.trie_key(),
            IpNet::V6(net) => net.trie_key(),
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
    fn network(&self) -> IpNet {
        IpNet::V4(*self)
    }

    fn address(&self) -> IpAddr {
        IpAddr::V4(self.addr())
    }

    fn trie_key(&self) -> Option<TrieKey> {
        Some(TrieKey::new(
            BitIp::Ipv4(self.network().to_bits()),
            self.network_prefix(),
        ))
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

    fn addr_string(&self) -> String {
        self.to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.addr()
    }
}

/// Implementation of the `BlockListNetwork` trait for IPv6 networks (`Ipv6Network`).
impl ListNetwork for Ipv6Net {
    fn network(&self) -> IpNet {
        IpNet::V6(*self)
    }

    fn address(&self) -> IpAddr {
        IpAddr::V6(self.addr())
    }

    fn trie_key(&self) -> Option<TrieKey> {
        Some(TrieKey::new(
            BitIp::Ipv6(self.network().to_bits()),
            self.network_prefix(),
        ))
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

    fn addr_string(&self) -> String {
        self.to_string()
    }

    fn is_network(&self) -> bool {
        self.network() == self.addr()
    }
}

impl ListNetwork for IpAsnRange {
    fn network(&self) -> IpNet {
        self.network.network()
    }

    fn address(&self) -> std::net::IpAddr {
        self.network.address()
    }

    fn trie_key(&self) -> Option<TrieKey> {
        self.network.trie_key()
    }

    fn network_prefix(&self) -> u8 {
        self.network.network_prefix()
    }

    fn addr_string(&self) -> String {
        self.network.addr_string()
    }

    fn is_network(&self) -> bool {
        self.network.is_network()
    }

    fn is_ipv4(&self) -> bool {
        self.network.is_ipv4()
    }

    fn is_ipv6(&self) -> bool {
        self.network.is_ipv6()
    }
}

impl ListNetwork for IpLocationRange {
    fn network(&self) -> IpNet {
        self.network.network()
    }

    fn address(&self) -> std::net::IpAddr {
        self.network.address()
    }

    fn trie_key(&self) -> Option<TrieKey> {
        self.network.trie_key()
    }

    fn network_prefix(&self) -> u8 {
        self.network.network_prefix()
    }

    fn addr_string(&self) -> String {
        self.network.addr_string()
    }

    fn is_network(&self) -> bool {
        self.network.is_network()
    }

    fn is_ipv4(&self) -> bool {
        self.network.is_ipv4()
    }

    fn is_ipv6(&self) -> bool {
        self.network.is_ipv6()
    }
}
