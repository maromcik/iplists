use std::{collections::HashMap, io::Cursor, net::IpAddr, time::Instant};

use flate2::read::GzDecoder;
use ipnetwork::IpNetwork;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    iplist::{
        config::IplistConfig,
        fetch::{Downloader, Loader},
        iprange::{IpAsnRange, IpLocationRange},
    },
    iptools::network::{ListNetwork, NetworkType},
};

pub struct Parser {
    pub body: Vec<u8>,
}

impl Parser {
    pub async fn parse<T: Serialize + for<'a> Deserialize<'a>>(&self) -> Result<Vec<T>, AppError> {
        let cursor = Cursor::new(&self.body);
        let decoder = GzDecoder::new(cursor);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(decoder);
        let mut data = Vec::new();
        for record in reader.deserialize() {
            let range: T = record?;
            data.push(range);
        }
        debug!("data parsed");
        Ok(data)
    }
}
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    #[serde(rename = "name")]
    pub country_name: String,
    #[serde(rename(serialize = "alpha2", deserialize = "alpha-2"))]
    pub country_alpha2: String,
    #[serde(rename(serialize = "continent", deserialize = "region"))]
    pub continent: String,
}

impl Location {
    pub fn load(config: &IplistConfig) -> Result<Vec<Self>, AppError> {
        let locations: Vec<Location> = csv::Reader::from_path(&config.location_path)?
            .deserialize()
            .collect::<Result<Vec<Location>, _>>()?;
        Ok(locations)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct IpLocationRangeOnly {
    pub start: IpAddr,
    pub end: IpAddr,
    pub country_alpha2: String,
}

impl IpLocationRangeOnly {
    pub async fn download(config: &IplistConfig) -> Result<Vec<Self>, AppError> {
        let filename = "ip-location.csv.gz";
        let parser = match Loader::new(&config.output_folder, filename).load().await {
            Ok(parser) => parser,
            Err(AppError::DataFileLoadError(e)) => {
                warn!("re-downloading file; cause: {}", e);
                Downloader::new(&config.country_uri, config.timeout, &config.headers)
                    .download()
                    .await?
                    .save(&config.output_folder, filename)
                    .await?
            }
            Err(e) => {
                return Err(e);
            }
        };

        parser.parse().await
    }

    pub async fn parse(
        config: &IplistConfig,
        locations: &Vec<Location>,
    ) -> Result<Vec<IpLocationRange>, AppError> {
        let ranges = IpLocationRangeOnly::download(config).await?;

        let t = Instant::now();

        debug!("parsing {} Location IP ranges", ranges.len());
        let mut location_map = HashMap::new();
        for location in locations {
            location_map.insert(location.country_alpha2.clone(), location);
        }
        let mut parsed_ranges = Vec::new();
        for range in ranges {
            if let Some(location) = location_map.get(&range.country_alpha2) {
                let start = IpNetwork::from_ip_addr(range.start).ok_or_else(|| {
                    AppError::ParseError(format!("unsupported start IP: {}", range.start))
                })?;
                let end = IpNetwork::from_ip_addr(range.end).ok_or_else(|| {
                    AppError::ParseError(format!("unsupported end IP: {}", range.end))
                })?;
                parsed_ranges.push(IpLocationRange {
                    network: NetworkType::Range(start, end),
                    location: (*location).to_owned(),
                });
            }
        }
        info!(
            "parsed {} Location IP ranges in {:?}",
            parsed_ranges.len(),
            t.elapsed()
        );

        Ok(parsed_ranges)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRangeOnly {
    pub start: IpAddr,
    pub end: IpAddr,
    pub asn: u32,
    pub isp: String,
}

impl From<IpAsnRangeOnly> for IpAsnRange {
    fn from(r: IpAsnRangeOnly) -> Self {
        IpAsnRange {
            network: NetworkType::Range(
                IpNetwork::from_ip_addr(r.start).expect("unsupported start IP"),
                IpNetwork::from_ip_addr(r.end).expect("unsupported end IP"),
            ),
            asn: r.asn,
            isp: r.isp,
        }
    }
}

impl IpAsnRangeOnly {
    pub async fn parse(config: &IplistConfig) -> Result<Vec<IpAsnRange>, AppError> {
        let filename = "ip-asn.csv.gz";
        let parser = match Loader::new(&config.output_folder, filename).load().await {
            Ok(parser) => parser,
            Err(AppError::DataFileLoadError(e)) => {
                warn!("re-downloading file; cause: {}", e);
                Downloader::new(&config.asn_uri, config.timeout, &config.headers)
                    .download()
                    .await?
                    .save(&config.output_folder, filename)
                    .await?
            }
            Err(e) => {
                return Err(e);
            }
        };

        let ranges: Vec<IpAsnRangeOnly> = parser.parse().await?;

        let processed_ranges: Vec<IpAsnRange> = ranges.into_iter().map(IpAsnRange::from).collect();

        info!("parsed {} ASN IP ranges", processed_ranges.len(),);

        Ok(processed_ranges)
    }
}
