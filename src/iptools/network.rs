use ipnet::{IpNet, Ipv4Net, Ipv6Net};
use std::{
    fmt::{Debug, Display},
    net::IpAddr,
    str::FromStr,
    sync::Arc,
};

use crate::{
    error::AppError,
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

pub trait Splitable {
    type Output: ListNetwork;
    fn split(&self) -> Result<(Self::Output, Self::Output), AppError>;
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

impl Splitable for IpNet {
    type Output = IpNet;

    fn split(&self) -> Result<(Self::Output, Self::Output), AppError> {
        match self {
            IpNet::V4(net) => {
                let (a, b) = Splitable::split(net)?;
                Ok((IpNet::V4(a), IpNet::V4(b)))
            }
            IpNet::V6(net) => {
                let (a, b) = Splitable::split(net)?;
                Ok((IpNet::V6(a), IpNet::V6(b)))
            }
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

impl Splitable for Ipv4Net {
    type Output = Ipv4Net;

    fn split(&self) -> Result<(Self::Output, Self::Output), AppError> {
        let mut subnets = self.subnets(self.prefix_len() + 1)?;
        let Some(a) = subnets.next() else {
            return Err(AppError::AddressManipulationError("No subnets".to_string()));
        };
        let Some(b) = subnets.next() else {
            return Err(AppError::AddressManipulationError("No subnets".to_string()));
        };
        Ok((a, b))
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

impl Splitable for Ipv6Net {
    type Output = Ipv6Net;

    fn split(&self) -> Result<(Self::Output, Self::Output), AppError> {
        let mut subnets = self.subnets(self.prefix_len() + 1)?;
        let Some(a) = subnets.next() else {
            return Err(AppError::AddressManipulationError("No subnets".to_string()));
        };
        let Some(b) = subnets.next() else {
            return Err(AppError::AddressManipulationError("No subnets".to_string()));
        };
        Ok((a, b))
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

impl Splitable for IpAsnRange {
    type Output = IpAsnRange;

    fn split(&self) -> Result<(Self::Output, Self::Output), AppError> {
        let (a, b) = self.network.split()?;
        Ok((
            IpAsnRange {
                network: a,
                asn: self.asn,
                isp: self.isp.clone(),
            },
            IpAsnRange {
                network: b,
                asn: self.asn,
                isp: self.isp.clone(),
            },
        ))
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

impl Splitable for IpLocationRange {
    type Output = IpLocationRange;

    fn split(&self) -> Result<(Self::Output, Self::Output), AppError> {
        let (a, b) = self.network.split()?;
        Ok((
            IpLocationRange {
                network: a,
                location: self.location.clone(),
            },
            IpLocationRange {
                network: b,
                location: self.location.clone(),
            },
        ))
    }
}

impl<T: ListNetwork> ListNetwork for Arc<T> {
    fn address(&self) -> IpAddr {
        (**self).address()
    }

    fn network_prefix(&self) -> u8 {
        (**self).network_prefix()
    }

    fn trie_key(&self) -> TrieKey {
        (**self).trie_key()
    }

    fn network_string(&self) -> String {
        (**self).network_string()
    }

    fn is_net(&self) -> bool {
        (**self).is_net()
    }

    fn is_ipv4(&self) -> bool {
        (**self).is_ipv4()
    }

    fn is_ipv6(&self) -> bool {
        (**self).is_ipv6()
    }
}

impl<T> Splitable for Arc<T>
where
    T: Splitable<Output = T> + ListNetwork,
{
    type Output = Arc<T>;

    fn split(&self) -> Result<(Self::Output, Self::Output), AppError> {
        let (a, b) = (**self).split()?;
        Ok((Arc::new(a), Arc::new(b)))
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
