//! Geo-IP / ASN pipeline: provider download and caching ([`fetch`]),
//! configuration ([`config`]), the in-memory indexes and provider traits
//! ([`iprange`]), provider implementations ([`parsers`]) and output
//! serialization ([`formatter`]).

pub mod config;
pub mod fetch;
pub mod formatter;
pub mod iprange;
pub mod parsers;
