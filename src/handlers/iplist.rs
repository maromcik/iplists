use crate::AppState;
use crate::error::AppError;
use crate::forms::IpVersion;
use crate::forms::extractors::AppQuery;
use crate::forms::iplist::{IpListFormByAsn, IpListFormByCountry};
use crate::iplist::iprange::{IpAsnRangeByIp, IpLocationRange, IpLocationRangeByIp};
use crate::iptools::network::ListNetwork;
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use ipnetwork::IpNetwork;
use itertools::Itertools;
use serde::Serialize;
use std::hash::Hash;
use std::sync::Arc;

pub async fn get_all_countries<T>(
    State(state): State<Arc<AppState<T>>>,
) -> Result<impl IntoResponse, AppError>
where
    T: ListNetwork + Clone + Eq + PartialEq + Serialize + Hash + Send + Sync,
{
    let countries = state.ip_ranges.read().await.locations.clone();
    Ok(Json(countries))
}

pub async fn get_all_continents<T>(
    State(state): State<Arc<AppState<T>>>,
) -> Result<impl IntoResponse, AppError>
where
    T: ListNetwork + Clone + Eq + PartialEq + Serialize + Hash + Send + Sync,
{
    let continents = state
        .ip_ranges
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

fn get_ips_by_version<T>(
    ips: Arc<IpLocationRangeByIp<T>>,
    form: &IpListFormByCountry,
) -> Vec<IpLocationRange<T>>
where
    T: ListNetwork + Clone + Eq + PartialEq + Serialize + Hash + Send + Sync,
{
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

pub async fn get_by_location<T>(
    State(state): State<Arc<AppState<T>>>,
    AppQuery(form): AppQuery<IpListFormByCountry>,
) -> Result<impl IntoResponse, AppError>
where
    T: ListNetwork + Clone + Eq + PartialEq + Serialize + Hash + Send + Sync,
{
    let formatted = if let Some(continent) = &form.continent {
        let ips = state
            .ip_ranges
            .read()
            .await
            .get_by_continent(continent)
            .await?;
        form.format
            .format(&get_ips_by_version(ips, &form), form.continent.as_deref())
    } else if let Some(country) = &form.country {
        let ips = state.ip_ranges.read().await.get_by_country(country).await?;
        form.format
            .format(&get_ips_by_version(ips, &form), form.country.as_deref())
    } else {
        let ips = state
            .ip_ranges
            .read()
            .await
            .location_ranges
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

pub async fn get_by_asn<T>(
    State(state): State<Arc<AppState<T>>>,
    AppQuery(form): AppQuery<IpListFormByAsn>,
) -> Result<impl IntoResponse, AppError>
where
    T: ListNetwork + Clone + Eq + PartialEq + Serialize + Hash + Send + Sync,
{
    let ips = if let Some(asn) = &form.asn {
        state.ip_ranges.read().await.get_by_asn(asn).await?
    } else {
        Arc::new(
            state
                .ip_ranges
                .read()
                .await
                .asn_ranges
                .by_asn
                .values()
                .fold(IpAsnRangeByIp::default(), |mut acc, v| {
                    acc.ipv4.extend(v.ipv4.iter().cloned());
                    acc.ipv6.extend(v.ipv6.iter().cloned());
                    acc
                }),
        )
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
