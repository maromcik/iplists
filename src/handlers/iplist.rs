use crate::error::AppError;
use crate::forms::iplist::{IpListFormByAsn, IpListFormByCountry};
use crate::iplist::iprange::IpRanges;
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn get_by_country(
    State(state): State<Arc<RwLock<IpRanges>>>,
    Query(form): Query<IpListFormByCountry>,
) -> Result<impl IntoResponse, AppError> {
    let ips = if let Some(continent) = &form.continent {
        state.read().await.get_by_continent(continent).await?
    } else if let Some(country) = &form.country {
        state.read().await.get_by_country(country).await?
    } else {
        state.read().await.location_ranges.all.clone()
    };
    let formatted = form.format.format(&ips, form.country.as_deref());
    Ok(formatted)
}

pub async fn get_by_asn(
    State(state): State<Arc<RwLock<IpRanges>>>,
    Query(form): Query<IpListFormByAsn>,
) -> Result<impl IntoResponse, AppError> {
    let ips = if let Some(asn) = &form.asn {
        state.read().await.get_by_asn(asn).await?
    } else {
        state.read().await.asn_ranges.clone()
    };
    let formatted = form
        .format
        .format(&ips, form.asn.map(|asn| asn.to_string()).as_deref());
    Ok(formatted)
}

pub async fn add_ip() -> Result<(), AppError> {
    Ok(())
}

pub async fn status() -> Result<impl IntoResponse, AppError> {
    Ok("ok")
}
