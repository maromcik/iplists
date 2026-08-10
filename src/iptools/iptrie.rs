use std::net::IpAddr;

use crate::error::AppError;
use crate::iptools::network::{ListNetwork, Splitable};

#[derive(Copy, Clone)]
pub struct TrieKey {
    pub addr: BitIp,
    pub network_prefix: u8,
}

impl TrieKey {
    pub fn new(addr: BitIp, network_prefix: u8) -> Self {
        Self {
            addr,
            network_prefix,
        }
    }

    /// Returns the i-th bit (from the MSB) of the key address.
    pub fn bit(&self, i: u8) -> u8 {
        let n = self.addr.max_prefix() - 1 - i;
        self.addr.r_shift(n).b_and(1)
    }
}

impl From<IpAddr> for TrieKey {
    fn from(addr: IpAddr) -> Self {
        let (bit_ip, prefix) = match addr {
            IpAddr::V4(addr) => (BitIp::Ipv4(addr.to_bits()), 32),
            IpAddr::V6(addr) => (BitIp::Ipv6(addr.to_bits()), 128),
        };

        Self::new(bit_ip, prefix)
    }
}

/// Represents a generic IP address in either IPv4 or IPv6 format using numeric representations.
#[allow(dead_code)]
#[derive(Copy, Clone)]
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
    pub fn r_shift(&self, n: u8) -> Self {
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
    pub fn b_and(self, rhs: u8) -> u8 {
        match self {
            BitIp::Ipv4(ip) => (ip & rhs as u32) as u8,
            BitIp::Ipv6(ip) => (ip & rhs as u128) as u8,
        }
    }

    /// Returns the maximum prefix length for the IP address.
    ///
    /// # Returns
    /// The maximum prefix length as an `u8` value.
    pub fn max_prefix(&self) -> u8 {
        match self {
            BitIp::Ipv4(_) => 32,
            BitIp::Ipv6(_) => 128,
        }
    }

    pub fn is_ipv4(&self) -> bool {
        matches!(self, BitIp::Ipv4(_))
    }

    pub fn is_ipv6(&self) -> bool {
        matches!(self, BitIp::Ipv6(_))
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
impl<T: ListNetwork + Splitable<Output = T>> TrieNode<T> {
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

            let key = network.trie_key();
            let bit = key.bit(i);
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
    fn lookup(&self, address: TrieKey) -> Option<&T> {
        let max_prefix = address.addr.max_prefix();

        let mut node = self;
        for i in 0..max_prefix {
            if let Some(ref value) = node.value {
                return if value.is_ipv4() == address.addr.is_ipv4() {
                    Some(value)
                } else {
                    None
                };
            }

            let bit = address.bit(i);
            node = node.children[bit as usize].as_ref()?;
        }

        node.value
            .as_ref()
            .filter(|&value| value.is_ipv4() == address.addr.is_ipv4())
    }

    /// Subtracts the networks stored in this subtree (set B) from `network`.
    ///
    /// `depth` is the node's distance from the root, i.e. how many bits of
    /// `network` have already been matched. Subnets of `network` that are not
    /// covered by any stored network are appended to `acc`.
    ///
    /// Relies on the insert-time invariant that a node either holds a value
    /// (with no children) or every child subtree contains a stored network.
    fn subtract(&self, network: &T, depth: u8, acc: &mut Vec<T>) -> Result<(), AppError> {
        // A stored same-family network on the walked path is a supernet
        // of (or equal to) `network`: it is fully covered, nothing remains.
        if let Some(stored) = self.value.as_ref()
            && stored.is_ipv4() == network.is_ipv4()
        {
            return Ok(());
        }

        // A host route cannot be split and cannot contain stored subnets.
        if depth == network.trie_key().addr.max_prefix() {
            acc.push(network.clone());
            return Ok(());
        }

        if depth == network.network_prefix() {
            // All bits consumed: B can only overlap via subnets strictly
            // inside `network`. If there are none, the whole stays.
            if self.children.iter().all(Option::is_none) {
                acc.push(network.clone());
                return Ok(());
            }
            // Partial overlap: split `network` and subtract each half from
            // the matching child subtree.
            let (a, b) = network.split()?;
            for half in [a, b] {
                let bit = half.trie_key().bit(depth) as usize;
                match &self.children[bit] {
                    Some(child) => child.subtract(&half, depth + 1, acc)?,
                    None => acc.push(half),
                }
            }
            return Ok(());
        }

        let bit = network.trie_key().bit(depth) as usize;
        match &self.children[bit] {
            Some(child) => child.subtract(network, depth + 1, acc),
            None => {
                // No stored network anywhere in this subtree: disjoint.
                acc.push(network.clone());
                Ok(())
            }
        }
    }
}

/// A prefix trie for IP networks that supports insertion and host-address lookup.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IPTrie<T: ListNetwork> {
    root: TrieNode<T>,
}

#[allow(dead_code)]
impl<T: ListNetwork + Splitable<Output = T>> IPTrie<T> {
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
    pub fn lookup(&self, address: TrieKey) -> Option<&T> {
        self.root.lookup(address)
    }

    /// Computes `network − B`: returns the subnets of `network` that are not
    /// covered by any network stored in this trie. The remainder is produced
    /// by splitting `network` only as far as needed.
    pub fn subtract(&self, network: &T) -> Result<Vec<T>, AppError> {
        let mut acc = Vec::new();
        self.root.subtract(network, 0, &mut acc)?;
        Ok(acc)
    }
}

impl<T: ListNetwork + Splitable<Output = T>> Default for IPTrie<T> {
    fn default() -> Self {
        Self::new()
    }
}

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
    T: ListNetwork + Splitable<Output = T>,
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
    T: ListNetwork + Splitable<Output = T>,
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
    use ipnet::IpNet;

    fn net(s: &str) -> IpNet {
        s.parse().unwrap()
    }

    fn subtract_all(trie: &IPTrie<IpNet>, a: &[&str]) -> Vec<IpNet> {
        let mut result = Vec::new();
        for s in a {
            result.extend(trie.subtract(&net(s)).unwrap());
        }
        result
    }

    #[test]
    fn subtract_example_from_issue() {
        // A = {1.1.1.1, 8.8.8.8, 192.168.0.0/24}
        // B = {8.8.8.8, 192.168.0.0/25}
        // A - B = {1.1.1.1, 192.168.0.128/25}
        let b = build(vec![net("8.8.8.8/32"), net("192.168.0.0/25")]);
        let result = subtract_all(&b, &["1.1.1.1/32", "8.8.8.8/32", "192.168.0.0/24"]);
        assert_eq!(result, vec![net("1.1.1.1/32"), net("192.168.0.128/25")]);
    }

    #[test]
    fn subtract_exact_match_is_covered() {
        let b = build(vec![net("192.168.0.0/24")]);
        assert!(subtract_all(&b, &["192.168.0.0/24"]).is_empty());
    }

    #[test]
    fn subtract_supernet_covers() {
        let b = build(vec![net("192.168.0.0/24")]);
        assert!(subtract_all(&b, &["192.168.0.0/25", "192.168.0.200/32"]).is_empty());
    }

    #[test]
    fn subtract_disjoint_keeps_whole() {
        let b = build(vec![net("192.168.0.0/24")]);
        assert_eq!(subtract_all(&b, &["10.0.0.0/8"]), vec![net("10.0.0.0/8")]);
    }

    #[test]
    fn subtract_nested_splits() {
        // A has 192.168.0.0/24, B has the first /25 and the first half of the second.
        let b = build(vec![net("192.168.0.0/25"), net("192.168.0.128/26")]);
        assert_eq!(
            subtract_all(&b, &["192.168.0.0/24"]),
            vec![net("192.168.0.192/26")]
        );
    }

    #[test]
    fn subtract_does_not_mutate_trie() {
        let b = build(vec![net("192.168.0.0/25")]);
        let first = subtract_all(&b, &["192.168.0.0/24"]);
        let second = subtract_all(&b, &["192.168.0.0/24"]);
        assert_eq!(first, second);
    }
}
