use crate::AppState;
use crate::error::AppError;
use crate::forms::extractors::AppQuery;
use crate::forms::iplist::{IpListFormByAsn, IpListFormByCountry};
use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
use itertools::Itertools;
use std::sync::Arc;

pub async fn get_all_countries(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
    let countries = state.ip_ranges.read().await.locations.clone();
    Ok(Json(countries))
}

pub async fn get_all_continents(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, AppError> {
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

pub async fn get_by_location(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<IpListFormByCountry>,
) -> Result<impl IntoResponse, AppError> {
    let formatted = if let Some(continent) = &form.continent {
        let ips = state
            .ip_ranges
            .read()
            .await
            .get_by_continent(continent)
            .await?;
        form.format.format(&ips, form.continent.as_deref())
    } else if let Some(country) = &form.country {
        let ips = state.ip_ranges.read().await.get_by_country(country).await?;
        form.format.format(&ips, form.country.as_deref())
    } else {
        let ips = state.ip_ranges.read().await.location_ranges.all.clone();
        form.format.format(&ips, None)
    };

    Ok(formatted)
}

pub async fn get_by_asn(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<IpListFormByAsn>,
) -> Result<impl IntoResponse, AppError> {
    let ips = if let Some(asn) = &form.asn {
        state.ip_ranges.read().await.get_by_asn(asn).await?
    } else {
        state.ip_ranges.read().await.asn_ranges.clone()
    };
    let formatted = form
        .format
        .format(&ips, form.asn.map(|asn| format!("asn{}", asn)).as_deref());
    Ok(formatted)
}
