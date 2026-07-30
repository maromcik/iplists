use std::collections::HashMap;
use std::time::Instant;

use ipnet::IpNet;
use log::{info, warn};
use serde::Deserialize;

use crate::iplist::iprange::{AsnParser, LocationParser};
use crate::iplist::iprange::{IpAsnRange, IpLocationRange, Location};
use crate::{
    error::AppError,
    iplist::{
        config::IplistConfig,
        fetch::{Downloader, GeoData, Loader},
    },
};

/// Filenames of the downloaded provider archives within the data folder.
const COUNTRY_FILENAME: &str = "ip-country.zip";
const ASN_FILENAME: &str = "ip-asn.csv.gz";

/// CSV filenames inside the provider archives.
const LOCATIONS_CSV: &str = "GeoLite2-Country-Locations-en.csv";
const COUNTRY_BLOCKS_V4_CSV: &str = "GeoLite2-Country-Blocks-IPv4.csv";
const COUNTRY_BLOCKS_V6_CSV: &str = "GeoLite2-Country-Blocks-IPv6.csv";
const ASN_BLOCKS_V4_CSV: &str = "GeoLite2-ASN-Blocks-IPv4.csv";
const ASN_BLOCKS_V6_CSV: &str = "GeoLite2-ASN-Blocks-IPv6.csv";

pub struct MaxMindParser;

async fn load_or_download(
    config: &IplistConfig,
    filename: &str,
    uri: &str,
) -> Result<GeoData, AppError> {
    match Loader::new(&config.output_folder, filename, config.max_age)
        .load()
        .await
    {
        Ok(parser) => Ok(parser),
        Err(AppError::DataFileLoadError(e)) => {
            warn!("re-downloading file; cause: {}", e);
            Downloader::new(
                uri,
                config.timeout,
                &config.headers,
                config.basic_auth.as_ref(),
            )
            .download()
            .await?
            .save(&config.output_folder, filename)
            .await
        }
        Err(e) => Err(e),
    }
}

/// One row of `GeoLite2-Country-Locations-en.csv`.
#[derive(Deserialize)]
struct MaxMindLocation {
    #[serde(rename(deserialize = "geoname_id"))]
    id: String,
    #[serde(rename(deserialize = "country_name"))]
    name: String,
    #[serde(rename(deserialize = "country_iso_code"))]
    code: String,
    #[serde(rename(deserialize = "continent_name"))]
    continent: String,
}

impl From<MaxMindLocation> for Location {
    fn from(row: MaxMindLocation) -> Self {
        Self {
            id: row.id,
            name: row.name,
            code: row.code,
            continent: row.continent,
        }
    }
}

/// One row of `GeoLite2-Country-Blocks-IPv*.csv`.
#[derive(Deserialize)]
struct MaxMindLocationBlock {
    #[serde(rename(deserialize = "geoname_id"))]
    id: String,
    network: IpNet,
}

/// One row of `GeoLite2-ASN-Blocks-IPv*.csv`.
#[derive(Deserialize)]
struct MaxMindAsnBlock {
    network: IpNet,
    #[serde(rename(deserialize = "autonomous_system_number"))]
    asn: u32,
    #[serde(rename(deserialize = "autonomous_system_organization"))]
    isp: String,
}

impl From<MaxMindAsnBlock> for IpAsnRange {
    fn from(row: MaxMindAsnBlock) -> Self {
        Self {
            network: row.network,
            asn: row.asn,
            isp: row.isp,
        }
    }
}

impl LocationParser for MaxMindParser {
    async fn locations(config: &IplistConfig) -> Result<Vec<Location>, AppError> {
        let archive = load_or_download(config, COUNTRY_FILENAME, &config.country_uri).await?;

        let rows: Vec<MaxMindLocation> = archive.csv_rows(LOCATIONS_CSV)?;
        Ok(rows.into_iter().map(Location::from).collect())
    }

    async fn location_ranges(
        config: &IplistConfig,
        locations: &[Location],
    ) -> Result<Vec<IpLocationRange>, AppError> {
        // The archive is freshly loaded by `locations()`, reuse it.
        let archive = Loader::new(&config.output_folder, COUNTRY_FILENAME, config.max_age)
            .load()
            .await?;

        let mut rows: Vec<MaxMindLocationBlock> = archive.csv_rows(COUNTRY_BLOCKS_V4_CSV)?;
        let rest = archive.csv_rows(COUNTRY_BLOCKS_V6_CSV)?;
        rows.extend(rest);

        info!("parsing {} Location IP ranges", rows.len());
        let t = Instant::now();

        let location_map: HashMap<&str, &Location> =
            locations.iter().map(|l| (l.id.as_str(), l)).collect();
        let mut parsed_ranges = Vec::new();
        for row in rows {
            if let Some(location) = location_map.get(row.id.as_str()) {
                parsed_ranges.push(IpLocationRange {
                    network: row.network,
                    location: (*location).clone(),
                });
            }
        }
        info!(
            "parsed {} Location IP subnets in {:?}",
            parsed_ranges.len(),
            t.elapsed()
        );

        Ok(parsed_ranges)
    }
}

impl AsnParser for MaxMindParser {
    async fn asn_ranges(config: &IplistConfig) -> Result<Vec<IpAsnRange>, AppError> {
        let archive = load_or_download(config, ASN_FILENAME, &config.asn_uri).await?;

        let mut rows: Vec<MaxMindAsnBlock> = archive.csv_rows(ASN_BLOCKS_V4_CSV)?;
        let rest = archive.csv_rows(ASN_BLOCKS_V6_CSV)?;
        rows.extend(rest);

        info!("parsing {} ASN IP ranges", rows.len());
        let t = Instant::now();

        let parsed_ranges = rows.into_iter().map(IpAsnRange::from).collect::<Vec<_>>();
        info!(
            "parsed {} ASN IP subnets in {:?}",
            parsed_ranges.len(),
            t.elapsed()
        );

        Ok(parsed_ranges)
    }
}
