use crate::AppState;
use crate::error::AppError;
use crate::forms::IpVersion;
use crate::forms::extractors::AppQuery;
use crate::forms::iplist::{ApiGeoLocation, IpListFormByAsn, IpListFormByCountry};
use crate::iplist::iprange::{IpAsnRangeByIp, IpLocationRange, IpLocationRangeByIp, Location};
use crate::iptools::iptrie::TrieKey;
use crate::models::iprange::CombinedIpRange;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use itertools::Itertools;
use std::net::IpAddr;
use std::sync::Arc;

/// `GET /api/iplist/country` — returns the full location table
/// (countries with their codes and continents) as JSON.
pub async fn get_all_countries(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let countries = state
        .ip_lists
        .location_ranges
        .read()
        .await
        .locations
        .clone();
    Ok(Json(countries))
}

/// `GET /api/iplist/continent` — returns the distinct continent names
/// present in the location table, as a JSON array of strings.
pub async fn get_all_continents(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let continents = state
        .ip_lists
        .location_ranges
        .read()
        .await
        .locations
        .iter()
        .cloned()
        .unique_by(|l| l.continent.clone())
        .map(|l| l.continent)
        .collect::<Vec<_>>();
    Ok(Json(continents))
}

/// Picks the requested IP family from a location range set.
///
/// `None` merges both families (v4 first) — the common case for callers
/// that want "all networks of this country".
fn get_ips_by_version(
    ips: Arc<IpLocationRangeByIp>,
    form: &IpListFormByCountry,
) -> Vec<Arc<IpLocationRange>> {
    match form.version {
        Some(IpVersion::Ipv4) => ips.ipv4.clone(),
        Some(IpVersion::Ipv6) => ips.ipv6.clone(),
        None => {
            let mut ips_all = Vec::new();
            ips_all.extend(ips.ipv4.iter().cloned());
            ips_all.extend(ips.ipv6.iter().cloned());
            ips_all
        }
    }
}

/// `GET /api/iplist/location?[country=XX|continent=NAME][&version=ipv4|ipv6][&format=...]`
///
/// With neither `country` nor `continent`, returns every known network.
/// The response body is produced by [`OutputFormat::format`](crate::iplist::formatter::OutputFormat::format),
/// so the same endpoint serves JSON, plain text or nftables output.
pub async fn get_by_location(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<IpListFormByCountry>,
) -> Result<impl IntoResponse, AppError> {
    let formatted = if let Some(continent) = &form.continent {
        let ips = state.ip_lists.get_by_continent(continent).await?;
        form.format
            .format(&get_ips_by_version(ips, &form), form.continent.as_deref())
    } else if let Some(country) = &form.country {
        let ips = state.ip_lists.get_by_country(country).await?;
        form.format
            .format(&get_ips_by_version(ips, &form), form.country.as_deref())
    } else {
        let ips = state
            .ip_lists
            .location_ranges
            .read()
            .await
            .by_continent
            .values()
            .fold(IpLocationRangeByIp::default(), |mut acc, v| {
                acc.ipv4.extend(v.ipv4.iter().cloned());
                acc.ipv6.extend(v.ipv6.iter().cloned());
                acc
            });
        form.format
            .format(&get_ips_by_version(Arc::new(ips), &form), None)
    };

    Ok(formatted)
}

/// `GET /api/iplist/asn?[asn=13335][&version=ipv4|ipv6][&format=...]`
///
/// With no `asn`, returns all known ASN ranges. The nftables set name is
/// derived from the ASN (`asn13335`) when one is requested.
pub async fn get_by_asn(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<IpListFormByAsn>,
) -> Result<impl IntoResponse, AppError> {
    let ips = if let Some(asn) = &form.asn {
        state.ip_lists.get_by_asn(asn).await?
    } else {
        Arc::new(state.ip_lists.asn_ranges.read().await.by_asn.values().fold(
            IpAsnRangeByIp::default(),
            |mut acc, v| {
                acc.ipv4.extend(v.ipv4.iter().cloned());
                acc.ipv6.extend(v.ipv6.iter().cloned());
                acc
            },
        ))
    };
    let ips = match form.version {
        Some(IpVersion::Ipv4) => ips.ipv4.clone(),
        Some(IpVersion::Ipv6) => ips.ipv6.clone(),
        None => {
            let mut ips_all = Vec::new();
            ips_all.extend(ips.ipv4.iter().cloned());
            ips_all.extend(ips.ipv6.iter().cloned());
            ips_all
        }
    };
    let formatted = form
        .format
        .format(&ips, form.asn.map(|asn| format!("asn{}", asn)).as_deref());
    Ok(formatted)
}

/// `GET /api/iplist/geo?ip=1.2.3.4`
///
/// Point lookup: resolves a single IP address against the location and ASN
/// longest-prefix-match tries and returns a combined JSON view. Whichever
/// side misses is filled with defaults; 404 only if both miss.
pub async fn geo_location(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<ApiGeoLocation>,
) -> Result<impl IntoResponse, AppError> {
    let key = TrieKey::from(form.ip);
    let locations_guard = state.ip_lists.location_ranges.read().await;
    let asns_guard = state.ip_lists.asn_ranges.read().await;
    let (location, asn) = match form.ip {
        IpAddr::V4(_) => (
            locations_guard.trie.ipv4.lookup(key),
            asns_guard.trie.ipv4.lookup(key),
        ),
        IpAddr::V6(_) => (
            locations_guard.trie.ipv6.lookup(key),
            asns_guard.trie.ipv6.lookup(key),
        ),
    };

    let result = match (location, asn) {
        (Some(location), None) => CombinedIpRange::new(
            form.ip,
            location.network,
            0,
            "".to_string(),
            location.location.as_ref().clone(),
        ),
        (Some(location), Some(asn)) => CombinedIpRange::new(
            form.ip,
            location.network,
            asn.asn,
            asn.isp.clone(),
            location.location.as_ref().clone(),
        ),
        (None, Some(asn)) => CombinedIpRange::new(
            form.ip,
            asn.network,
            asn.asn,
            asn.isp.clone(),
            Location::default(),
        ),
        _ => return Err(AppError::NotFound("No location found".to_string())),
    };

    Ok(Json(result))
}
