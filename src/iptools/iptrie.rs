use crate::iptools::network::ListNetwork;

/// Represents a node in a prefix trie structure.
/// Each node tracks whether it's part of a subnet (`is_subnet`)
/// and has two children corresponding to binary bits (0 or 1).
#[derive(Default)]
struct TrieNode {
    children: [Option<Box<TrieNode>>; 2],
    is_subnet: bool,
}

impl TrieNode {
    /// Creates a new and defaulted `TrieNode` instance.
    ///
    /// # Returns
    /// A fresh `TrieNode` with no children and `is_subnet` set to `false`.
    fn new() -> Self {
        Self {
            children: Default::default(),
            is_subnet: false,
        }
    }

    /// Inserts an IP prefix into the trie. The path through the trie is determined
    /// by iterating over the bits of the IP address.
    ///
    /// # Parameters
    /// - `ip`: An instance of the `BlockListNetwork` trait, representing the IP prefix to insert.
    ///
    /// # Returns
    /// `true` if the prefix was successfully inserted, or `false` if it was already covered by
    /// an existing broader subnet.
    fn insert<T>(&mut self, ip: &T) -> bool
    where
        T: ListNetwork,
    {
        let mut node = self;

        for i in 0..ip.network_prefix() {
            if node.is_subnet {
                // This subnet is already covered by a broader one
                return false;
            }

            let n = ip.max_prefix() - 1 - i;
            let bit = ip.network_addr().r_shift(n).b_and(1);
            node = node.children[bit as usize].get_or_insert_with(|| Box::new(TrieNode::new()));
        }

        if node.is_subnet {
            // Exact subnet already exists — this is a duplicate.
            return false;
        }

        // Mark this node as a subnet and prune deeper subnets
        node.is_subnet = true;
        node.children = Default::default(); // Drop more specific subnets
        true
    }
}

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
#[must_use]
pub fn deduplicate<T>(ips: Option<Vec<T>>) -> Option<Vec<T>>
where
    T: ListNetwork,
{
    let mut ips = ips?;
    ips.sort_by_key(ListNetwork::network_prefix);
    let mut root = TrieNode::new();
    let mut result = Vec::new();
    for ip in ips {
        if root.insert(&ip) {
            result.push(ip);
        }
    }
    Some(result)
}
