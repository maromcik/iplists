//! Query/form types for the public API, plus custom axum extractors that
//! map deserialization failures onto the crate-wide
//! [`AppError`](crate::error::AppError).

use serde::Deserialize;

pub mod blocklist;
pub mod extractors;
pub mod iplist;

/// IP family selector accepted as the `version` query parameter
/// (case-sensitive lowercase: `ipv4` / `ipv6`).
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Copy, Hash)]
#[serde(rename_all = "lowercase")]
pub enum IpVersion {
    Ipv4,
    Ipv6,
}
