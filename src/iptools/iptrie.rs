use crate::iptools::network::ListNetwork;
use std::net::IpAddr;

/// Represents a generic IP address in either IPv4 or IPv6 format using numeric representations.
#[allow(dead_code)]
pub enum BitIp {
    Ipv4(u32),
    Ipv6(u128),
}

#[allow(dead_code)]
impl BitIp {
    /// Performs a right-shift operation on an IP address by `n` bits
    /// and returns the result in the corresponding `BitIp` format.
    ///
    /// # Parameters
    /// - `n`: The number of bits to shift.
    ///
    /// # Returns
    /// The shifted `BitIp` instance.
    fn r_shift(&self, n: u8) -> Self {
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
    fn b_and(self, rhs: u8) -> u8 {
        match self {
            BitIp::Ipv4(ip) => (ip & rhs as u32) as u8,
            BitIp::Ipv6(ip) => (ip & rhs as u128) as u8,
        }
    }
}

/// Represents a node in a prefix trie structure.
/// Each node optionally stores a network value and has two children
/// corresponding to binary bits (0 or 1).
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TrieNode<T: ListNetwork> {
    children: [Option<Box<TrieNode<T>>>; 2],
    value: Option<T>,
}

impl<T: ListNetwork> Default for TrieNode<T> {
    fn default() -> Self {
        Self {
            children: Default::default(),
            value: None,
        }
    }
}

#[allow(dead_code)]
impl<T: ListNetwork> TrieNode<T> {
    /// Creates a new and defaulted `TrieNode` instance.
    ///
    /// # Returns
    /// A fresh `TrieNode` with no children and no stored value.
    fn new() -> Self {
        Self::default()
    }

    /// Inserts an IP prefix into the trie. The path through the trie is determined
    /// by iterating over the bits of the IP address.
    ///
    /// # Parameters
    /// - `network`: A reference to the `ListNetwork` value to insert.
    ///
    /// # Returns
    /// `true` if the prefix was successfully inserted, or `false` if it was already covered by
    /// an existing broader subnet or is an exact duplicate.
    fn insert(&mut self, network: &T) -> bool {
        let mut node = self;

        for i in 0..network.network_prefix() {
            if node.value.is_some() {
                // This subnet is already covered by a broader one
                return false;
            }

            let n = network.max_prefix() - 1 - i;
            let bit = network.bit_network_addr().r_shift(n).b_and(1);
            node = node.children[bit as usize].get_or_insert_with(|| Box::new(TrieNode::new()));
        }

        if node.value.is_some() {
            // Exact subnet already exists — this is a duplicate.
            return false;
        }

        // Store the network at this node and prune deeper subnets
        node.value = Some(network.clone());
        node.children = Default::default();
        true
    }

    /// Looks up the network that contains the given host address.
    ///
    /// The address is treated as a single host (/32 for IPv4, /128 for IPv6).
    fn lookup(&self, address: IpAddr) -> Option<&T> {
        let bit_ip = address.bit_network_addr();
        let max_prefix = address.max_prefix();

        let mut node = self;
        for i in 0..max_prefix {
            if let Some(ref value) = node.value {
                return if value.is_ipv4() == address.is_ipv4() {
                    Some(value)
                } else {
                    None
                };
            }

            let n = max_prefix - 1 - i;
            let bit = bit_ip.r_shift(n).b_and(1);
            node = node.children[bit as usize].as_ref()?;
        }

        node.value
            .as_ref()
            .filter(|&value| value.is_ipv4() == address.is_ipv4())
    }
}

/// A prefix trie for IP networks that supports insertion and host-address lookup.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IPTrie<T: ListNetwork> {
    root: TrieNode<T>,
}

#[allow(dead_code)]
impl<T: ListNetwork> IPTrie<T> {
    /// Creates an empty `IPTrie`.
    pub fn new() -> Self {
        Self {
            root: TrieNode::new(),
        }
    }

    /// Inserts a network into the trie.
    ///
    /// Returns `true` if the network was inserted, or `false` if it was already covered by
    /// an existing broader network or is an exact duplicate.
    pub fn insert(&mut self, network: &T) -> bool {
        self.root.insert(network)
    }

    /// Looks up the network that contains the given host address.
    ///
    /// The address is interpreted as a single host (/32 for IPv4, /128 for IPv6). Returns
    /// a reference to the broadest matching network stored in the trie, or `None` if no
    /// network contains the address.
    pub fn lookup(&self, address: IpAddr) -> Option<&T> {
        self.root.lookup(address)
    }
}

impl<T: ListNetwork> Default for IPTrie<T> {
    fn default() -> Self {
        Self::new()
    }
}

// original version before auto-merge
// #[must_use]
// pub fn deduplicate<T>(ips: Option<Vec<T>>) -> Option<Vec<T>>
// where
//     T: ListNetwork,
// {
//     let mut ips = ips?;
//     ips.sort_by_key(ListNetwork::network_prefix);
//     let mut root = TrieNode::new();
//     let mut result = Vec::new();
//     for ip in ips {
//         if root.insert(&ip) {
//             result.push(ip);
//         }
//     }
//     Some(result)
// }

// bypass deduplication
// #[must_use]
// pub fn deduplicate<T>(ips: Option<Vec<NetworkType<T>>>) -> Option<Vec<NetworkType<T>>>
// where
//     T: ListNetwork,
// {
//     ips
// }

/// deduplicate only IPs/Subnets not ranges
/// Deduplicates a collection of IP prefixes by using a prefix trie to organize them.
/// This ensures that redundant, more specific subnets are removed.
/// For example, `192.168.0.0/16` will absorb `192.168.1.0/24`.
///
/// # Parameters
/// - `ips`: An iterator over IP prefixes that implements the `BlockListNetwork` trait.
///
/// # Returns
/// A deduplicated `Vec` containing the broadest possible subnets.
///
/// # Time Complexity
/// -   `O(h * n * logn)`: Sorting the IPs contributes `n * logn`, and inserting into the trie has
///     a height-dependent complexity of `h`, which is 32 for IPv4 and 128 for IPv6.
pub fn deduplicate<T>(ips: Vec<T>) -> Vec<T>
where
    T: ListNetwork,
{
    let mut networks = ips;
    networks.sort_by_key(ListNetwork::network_prefix);
    let mut root = TrieNode::new();
    let mut result = Vec::new();
    for ip in networks {
        if root.insert(&ip) {
            result.push(ip);
        }
    }
    result
}

pub fn build<T>(ips: Vec<T>) -> IPTrie<T>
where
    T: ListNetwork,
{
    let mut networks = ips;
    networks.sort_by_key(ListNetwork::network_prefix);
    let mut root = TrieNode::new();
    for ip in networks {
        root.insert(&ip);
    }
    IPTrie { root }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;

    #[test]
    fn lookup_ipv4_contained() {
        let mut trie = IPTrie::new();
        let net = ipnetwork::Ipv4Network::from_str("10.0.0.0/8").unwrap();
        assert!(trie.insert(&net));
        let addr = IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3));
        assert_eq!(trie.lookup(addr), Some(&net));
    }

    #[test]
    fn lookup_ipv4_not_contained() {
        let mut trie = IPTrie::new();
        let net = ipnetwork::Ipv4Network::from_str("10.0.0.0/8").unwrap();
        assert!(trie.insert(&net));
        let addr = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        assert_eq!(trie.lookup(addr), None);
    }

    #[test]
    fn lookup_ipv4_broader_wins() {
        let mut trie = IPTrie::new();
        let broad = ipnetwork::Ipv4Network::from_str("10.0.0.0/8").unwrap();
        let specific = ipnetwork::Ipv4Network::from_str("10.1.0.0/16").unwrap();
        assert!(trie.insert(&broad));
        assert!(!trie.insert(&specific));
        let addr = IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3));
        assert_eq!(trie.lookup(addr), Some(&broad));
    }

    #[test]
    fn lookup_ipv6_contained() {
        let mut trie = IPTrie::new();
        let net = ipnetwork::Ipv6Network::from_str("2001:db8::/32").unwrap();
        assert!(trie.insert(&net));
        let addr = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert_eq!(trie.lookup(addr), Some(&net));
    }

    #[test]
    fn lookup_ipv6_exact() {
        let mut trie = IPTrie::new();
        let net = ipnetwork::Ipv6Network::from_str("2001:db8::1/128").unwrap();
        assert!(trie.insert(&net));
        let addr = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        assert_eq!(trie.lookup(addr), Some(&net));
    }

    #[test]
    fn lookup_family_mismatch() {
        let mut trie = IPTrie::new();
        let net = ipnetwork::Ipv4Network::from_str("10.0.0.0/8").unwrap();
        assert!(trie.insert(&net));
        let addr = IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0xffff, 0x0a00, 0x0102));
        assert_eq!(trie.lookup(addr), None);
    }
}
