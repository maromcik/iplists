use log::{info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::time::Instant;

use crate::iplist::fetch::{Downloader, Loader};
use crate::iplist::formatter::OutputFormat;
use crate::{error::AppError, iplist::config::IplistConfig};

pub trait BaseIpRange {
    fn start(&self) -> IpAddr;
    fn end(&self) -> IpAddr;
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct IpCountryRangeOnly {
    pub start: IpAddr,
    pub end: IpAddr,
    pub country_alpha2: String,
}

impl IpCountryRangeOnly {
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
pub struct IpLocationRange {
    pub start: IpAddr,
    pub end: IpAddr,
    pub country_name: String,
    pub country_alpha2: String,
    pub continent: String,
}

impl BaseIpRange for IpLocationRange {
    fn start(&self) -> IpAddr {
        self.start
    }

    fn end(&self) -> IpAddr {
        self.end
    }
}

impl IpLocationRange {
    pub async fn parse(
        config: &IplistConfig,
        locations: &Vec<Location>,
    ) -> Result<Vec<Self>, AppError> {
        let ranges = IpCountryRangeOnly::download(config).await?;

        let t = Instant::now();

        info!("Parsing {} Location IP ranges", ranges.len());
        let mut location_map = HashMap::new();
        for location in locations {
            location_map.insert(location.country_alpha2.clone(), location);
        }
        let mut parsed_ranges = Vec::new();
        for range in ranges {
            if let Some(location) = location_map.get(&range.country_alpha2) {
                parsed_ranges.push(IpLocationRange {
                    start: range.start,
                    end: range.end,
                    country_name: location.country_name.clone(),
                    country_alpha2: location.country_alpha2.clone(),
                    continent: location.continent.clone(),
                });
            }
        }
        info!(
            "Parsed {} Location IP ranges in {:?}",
            parsed_ranges.len(),
            t.elapsed()
        );

        Ok(parsed_ranges)
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq, Hash)]
pub struct IpAsnRange {
    pub start: IpAddr,
    pub end: IpAddr,
    pub asn: u32,
    pub isp: String,
}

impl BaseIpRange for IpAsnRange {
    fn start(&self) -> IpAddr {
        self.start
    }

    fn end(&self) -> IpAddr {
        self.end
    }
}

impl IpAsnRange {
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

        let ranges = parser.parse().await?;

        info!("Parsed {} ASN IP ranges", ranges.len(),);

        Ok(ranges)
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRanges {
    pub all: Arc<Vec<IpAsnRange>>,
    pub by_asn: HashMap<u32, Arc<Vec<IpAsnRange>>>,
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRanges {
    pub all: Arc<Vec<IpLocationRange>>,
    pub by_country: HashMap<String, Arc<Vec<IpLocationRange>>>,
    pub by_continent: HashMap<String, Arc<Vec<IpLocationRange>>>,
}

impl IpLocationRanges {
    pub async fn save(&self, config: &IplistConfig) -> Result<(), AppError> {
        tokio::fs::create_dir_all(format!("{}/{}", config.output_folder, "gen")).await?;
        for (country, ranges) in &self.by_country {
            let path = format!("{}/gen/{}", config.output_folder, country);
            tokio::fs::write(
                format!("{path}.txt"),
                OutputFormat::Text.format(ranges, Some(country)).to_string(),
            )
            .await?;
            tokio::fs::write(
                format!("{path}.nft"),
                OutputFormat::Nftables
                    .format(ranges, Some(country))
                    .to_string(),
            )
            .await?;
        }
        info!("Country files saved");
        for (continent, ranges) in &self.by_continent {
            let path = format!("{}/gen/{}", config.output_folder, continent);
            tokio::fs::write(
                format!("{path}.txt"),
                OutputFormat::Text
                    .format(ranges, Some(continent))
                    .to_string(),
            )
            .await?;
            tokio::fs::write(
                format!("{path}.nft"),
                OutputFormat::Nftables
                    .format(ranges, Some(continent))
                    .to_string(),
            )
            .await?;
        }
        info!("Continent files saved");
        Ok(())
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpRanges {
    pub location_ranges: IpLocationRanges,
    pub asn_ranges: IpAsnRanges,
    pub locations: Arc<Vec<Location>>,
}

impl IpRanges {
    pub fn new(
        location_ranges: Vec<IpLocationRange>,
        asn_ranges: Vec<IpAsnRange>,
        locations: Vec<Location>,
    ) -> Self {
        let mut location_ranges_by_country: HashMap<String, Vec<IpLocationRange>> = HashMap::new();
        let mut location_ranges_by_continent: HashMap<String, Vec<IpLocationRange>> =
            HashMap::new();
        for range in &location_ranges {
            location_ranges_by_country
                .entry(range.country_alpha2.clone())
                .or_default()
                .push(range.clone());
            location_ranges_by_continent
                .entry(range.continent.clone())
                .or_default()
                .push(range.clone());
        }

        let mut asn_ranges_by_asn: HashMap<u32, Vec<IpAsnRange>> = HashMap::new();
        for range in &asn_ranges {
            asn_ranges_by_asn
                .entry(range.asn)
                .or_default()
                .push(range.clone());
        }

        Self {
            location_ranges: IpLocationRanges {
                all: Arc::new(location_ranges),
                by_country: location_ranges_by_country
                    .into_iter()
                    .map(|(k, v)| (k, Arc::new(v)))
                    .collect(),
                by_continent: location_ranges_by_continent
                    .into_iter()
                    .map(|(k, v)| (k, Arc::new(v)))
                    .collect(),
            },
            asn_ranges: IpAsnRanges {
                all: Arc::new(asn_ranges),
                by_asn: asn_ranges_by_asn
                    .into_iter()
                    .map(|(k, v)| (k, Arc::new(v)))
                    .collect(),
            },
            locations: Arc::new(locations),
        }
    }

    pub async fn get_by_continent(
        &self,
        continent: &str,
    ) -> Result<Arc<Vec<IpLocationRange>>, AppError> {
        let Some(ranges) = self.location_ranges.by_continent.get(continent) else {
            return Ok(Arc::new(vec![]));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_country(
        &self,
        country_alpha2: &str,
    ) -> Result<Arc<Vec<IpLocationRange>>, AppError> {
        let Some(ranges) = self.location_ranges.by_country.get(country_alpha2) else {
            return Ok(Arc::new(vec![]));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_asn(&self, asn: &u32) -> Result<Arc<Vec<IpAsnRange>>, AppError> {
        let Some(ranges) = self.asn_ranges.by_asn.get(asn) else {
            return Ok(Arc::new(vec![]));
        };
        Ok(ranges.clone())
    }

    pub fn get_by_country_name(
        &self,
        country_name_query: &str,
    ) -> impl Iterator<Item = &IpLocationRange> {
        self.location_ranges.all.iter().filter(|r| {
            r.country_name
                .to_lowercase()
                .contains(country_name_query.to_lowercase().as_str())
        })
    }
}

pub async fn generate_ranges(config: &IplistConfig) -> Result<IpRanges, AppError> {
    let locations = Location::load(config)?;
    let location_ranges = IpLocationRange::parse(config, &locations).await?;
    let asn_ranges = IpAsnRange::parse(config).await?;
    let ip_ranges = IpRanges::new(location_ranges, asn_ranges, locations);
    ip_ranges.location_ranges.save(config).await?;
    Ok(ip_ranges)
}
