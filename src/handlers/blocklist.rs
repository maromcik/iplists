use crate::AppState;
use crate::error::AppError;
use crate::forms::IpVersion;
use crate::forms::blocklist::BlocklistIpVersion;
use crate::forms::extractors::AppQuery;
use axum::extract::State;
use axum::response::IntoResponse;
use ipnet::IpNet;
use std::sync::Arc;

/// `GET /api/blocklist?[version=ipv4|ipv6][&format=...]`
///
/// Serves the merged, allowlist-filtered blocklist. With no `version`, IPv4
/// and IPv6 networks are wrapped into a single vector as `IpNet` so both
/// families appear in one response. The nftables set name is `blocklist`.
pub async fn get_blocklist(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<BlocklistIpVersion>,
) -> Result<impl IntoResponse, AppError> {
    let blocklist = state.blocklist.read().await;

    let formatted = match form.version {
        Some(IpVersion::Ipv4) => form.format.format(&blocklist.ipv4, Some("blocklist")),
        Some(IpVersion::Ipv6) => form.format.format(&blocklist.ipv6, Some("blocklist")),
        None => {
            let ips = blocklist
                .ipv4
                .iter()
                .map(|net| IpNet::from(*net))
                .chain(blocklist.ipv6.iter().map(|net| IpNet::from(*net)))
                .collect::<Vec<_>>();
            form.format.format(&ips, Some("blocklist"))
        }
    };

    Ok(formatted)
}
