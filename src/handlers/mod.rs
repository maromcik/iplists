//! Axum request handlers for the public API. Every handler reads from the
//! shared [`AppState`](crate::AppState) and renders responses through
//! [`OutputFormat`](crate::iplist::formatter::OutputFormat) where a list
//! is returned.

pub mod blocklist;
pub mod iplist;
pub mod status;
