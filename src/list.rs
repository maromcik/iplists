use crate::AppState;
use crate::iplist::iprange::{
    AsnParser, IpAsnRangeByIp, IpAsnRanges, IpLocationRange, IpLocationRangeByIp, IpLocationRanges,
    Location, LocationParser,
};
use crate::{error::AppError, iplist::config::IplistConfig};
use itertools::Itertools;
use log::error;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default)]
pub struct IpLists {
    pub location_ranges: RwLock<IpLocationRanges>,
    pub asn_ranges: RwLock<IpAsnRanges>,
}

impl IpLists {
    pub async fn get_by_continent(
        &self,
        continent: &str,
    ) -> Result<Arc<IpLocationRangeByIp>, AppError> {
        match self
            .location_ranges
            .read()
            .await
            .by_continent
            .get(continent)
        {
            Some(ranges) => Ok(ranges.clone()),
            None => Err(AppError::NotFound(format!(
                "No location ranges found for continent: {}",
                continent
            ))),
        }
    }

    pub async fn get_by_country(
        &self,
        country_alpha2: &str,
    ) -> Result<Arc<IpLocationRangeByIp>, AppError> {
        match self
            .location_ranges
            .read()
            .await
            .by_country
            .get(country_alpha2)
        {
            Some(ranges) => Ok(ranges.clone()),
            None => Err(AppError::NotFound(format!(
                "No location ranges found for country: {}",
                country_alpha2
            ))),
        }
    }

    pub async fn get_by_asn(&self, asn: &u32) -> Result<Arc<IpAsnRangeByIp>, AppError> {
        match self.asn_ranges.read().await.by_asn.get(asn) {
            Some(ranges) => Ok(ranges.clone()),
            None => Err(AppError::NotFound(format!(
                "No ASN ranges found for ASN: {}",
                asn
            ))),
        }
    }
}

pub async fn load_locations<P>(
    config: &IplistConfig,
) -> Result<(Vec<Location>, Vec<IpLocationRange>), AppError>
where
    P: LocationParser + AsnParser,
{
    let locations = P::locations(config)
        .await
        .map_err(|e| AppError::ListLoadError(format!("Failed to load locations: {e}")))?
        .into_iter()
        .filter(|l| !l.code.is_empty())
        .sorted_by_key(|l| l.code.clone())
        .collect::<Vec<_>>();
    let location_ranges = P::location_ranges(config, &locations)
        .await
        .map_err(|e| AppError::ListLoadError(format!("Failed to load countries: {e}")))?;
    Ok((locations, location_ranges))
}
pub async fn update_ranges<P>(state: Arc<AppState>)
where
    P: LocationParser + AsnParser,
{
    let mut geo_ok: bool = true;
    match load_locations::<P>(&state.config.iplist).await {
        Ok((locations, locations_parsed)) => {
            let locations_ranges = IpLocationRanges::new(locations, locations_parsed);
            if let Err(e) = locations_ranges.save(&state.config.iplist).await {
                let msg = format!("Failed to save locations: {e}");
                error!("{msg}");
                let mut status = state.status.write().await;
                status.locations.warning(msg);
            }
            *state.ip_lists.location_ranges.write().await = locations_ranges;
            state
                .status
                .write()
                .await
                .locations
                .ok("Locations loaded successfully");
        }

        Err(e) => {
            error!("{e}");
            state.status.write().await.locations.error(e.to_string());
            state
                .status
                .write()
                .await
                .geo
                .warning(format!("Locations not updated: {e}"));
            geo_ok = false;
        }
    };

    match P::asn_ranges(&state.config.iplist)
        .await
        .map_err(|e| AppError::ListLoadError(format!("Failed to load ASNs: {e}")))
    {
        Ok(asns_parsed) => {
            let asn_ranges = IpAsnRanges::new(asns_parsed);
            *state.ip_lists.asn_ranges.write().await = asn_ranges;
            state
                .status
                .write()
                .await
                .asns
                .ok("ASNs loaded successfully");
        }
        Err(e) => {
            error!("{e}");
            state.status.write().await.asns.error(e.to_string());
            state
                .status
                .write()
                .await
                .geo
                .warning(format!("ASNs not updated: {e}"));
            geo_ok = false;
        }
    };

    if geo_ok {
        state
            .status
            .write()
            .await
            .geo
            .ok("GEO index loaded successfully");
    }
}
