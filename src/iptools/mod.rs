//! Generic IP-network toolkit: the [`ListNetwork`](network::ListNetwork)
//! abstraction over v4/v6 networks, network splitting
//! ([`Splitable`](network::Splitable)) and a binary prefix trie
//! ([`IPTrie`](iptrie::IPTrie)) with longest-prefix lookup, set
//! subtraction and deduplication.

pub mod iptrie;
pub mod network;
