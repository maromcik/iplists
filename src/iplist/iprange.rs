use log::{debug, info, warn};
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
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct IpLocationRange {
    pub start: IpAddr,
    pub end: IpAddr,
    pub location: Location,
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
                parsed_ranges.push(IpLocationRange {
                    start: range.start,
                    end: range.end,
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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
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

        info!("parsed {} ASN IP ranges", ranges.len(),);

        Ok(ranges)
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRangeByIp {
    pub ipv4: Vec<IpAsnRange>,
    pub ipv6: Vec<IpAsnRange>,
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRanges {
    pub by_asn: HashMap<u32, Arc<IpAsnRangeByIp>>,
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRangeByIp {
    pub ipv4: Vec<IpLocationRange>,
    pub ipv6: Vec<IpLocationRange>,
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRanges {
    pub by_country: HashMap<String, Arc<IpLocationRangeByIp>>,
    pub by_continent: HashMap<String, Arc<IpLocationRangeByIp>>,
}

pub async fn save_data<T>(
    data: &[T],
    output: OutputFormat,
    path: &str,
    set_name: Option<&str>,
) -> Result<(), AppError>
where
    T: Serialize + BaseIpRange,
{
    tokio::fs::write(path, output.format(data, set_name).to_string()).await?;
    Ok(())
}

impl IpLocationRanges {
    pub async fn save(&self, config: &IplistConfig) -> Result<(), AppError> {
        tokio::fs::create_dir_all(format!("{}/{}", config.output_folder, "gen")).await?;
        for (country, ranges) in &self.by_country {
            let path = format!("{}/gen/{}", config.output_folder, country);
            let mut merged = ranges.ipv4.clone();
            merged.extend(ranges.ipv6.clone());
            save_data(
                &ranges.ipv4,
                OutputFormat::Text,
                &format!("{path}-ipv4.txt"),
                Some(country),
            )
            .await?;
            save_data(
                &ranges.ipv6,
                OutputFormat::Text,
                &format!("{path}-ipv6.txt"),
                Some(country),
            )
            .await?;
            save_data(
                &merged,
                OutputFormat::Text,
                &format!("{path}.txt"),
                Some(country),
            )
            .await?;
            save_data(
                &ranges.ipv4,
                OutputFormat::Nftables,
                &format!("{path}-ipv4.nft"),
                Some(country),
            )
            .await?;
            save_data(
                &ranges.ipv6,
                OutputFormat::Nftables,
                &format!("{path}-ipv6.nft"),
                Some(country),
            )
            .await?;
            save_data(
                &merged,
                OutputFormat::Nftables,
                &format!("{path}.nft"),
                Some(country),
            )
            .await?;
        }
        debug!("country files saved");
        for (continent, ranges) in &self.by_continent {
            let path = format!("{}/gen/{}", config.output_folder, continent);
            let mut merged = ranges.ipv4.clone();
            merged.extend(ranges.ipv6.clone());
            save_data(
                &merged,
                OutputFormat::Text,
                &format!("{path}.txt"),
                Some(continent),
            )
            .await?;
            save_data(
                &ranges.ipv4,
                OutputFormat::Text,
                &format!("{path}-ipv4.txt"),
                Some(continent),
            )
            .await?;
            save_data(
                &ranges.ipv6,
                OutputFormat::Text,
                &format!("{path}-ipv6.txt"),
                Some(continent),
            )
            .await?;
            save_data(
                &merged,
                OutputFormat::Nftables,
                &format!("{path}.nft"),
                Some(continent),
            )
            .await?;
            save_data(
                &ranges.ipv4,
                OutputFormat::Nftables,
                &format!("{path}-ipv4.nft"),
                Some(continent),
            )
            .await?;
            save_data(
                &ranges.ipv6,
                OutputFormat::Nftables,
                &format!("{path}-ipv6.nft"),
                Some(continent),
            )
            .await?;
        }
        debug!("continent files saved");
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
        let mut location_ranges_by_country: HashMap<String, IpLocationRangeByIp> = HashMap::new();
        let mut location_ranges_by_continent: HashMap<String, IpLocationRangeByIp> = HashMap::new();
        for range in location_ranges {
            match (range.start.is_ipv4(), range.end.is_ipv4()) {
                (true, true) => {
                    location_ranges_by_country
                        .entry(range.location.country_alpha2.clone())
                        .or_default()
                        .ipv4
                        .push(range.clone());
                    location_ranges_by_continent
                        .entry(range.location.continent.clone())
                        .or_default()
                        .ipv4
                        .push(range);
                }
                (false, false) => {
                    location_ranges_by_country
                        .entry(range.location.country_alpha2.clone())
                        .or_default()
                        .ipv6
                        .push(range.clone());
                    location_ranges_by_continent
                        .entry(range.location.continent.clone())
                        .or_default()
                        .ipv6
                        .push(range);
                }
                _ => {
                    warn!("unexpected IP range: {:?}", range);
                }
            }
        }

        let mut asn_ranges_by_asn: HashMap<u32, IpAsnRangeByIp> = HashMap::new();
        for range in &asn_ranges {
            match (range.start.is_ipv4(), range.end.is_ipv4()) {
                (true, true) => {
                    asn_ranges_by_asn
                        .entry(range.asn)
                        .or_default()
                        .ipv4
                        .push(range.clone());
                }
                (false, false) => {
                    asn_ranges_by_asn
                        .entry(range.asn)
                        .or_default()
                        .ipv6
                        .push(range.clone());
                }
                _ => {
                    warn!("unexpected IP range: {:?}", range);
                }
            }
        }
        info!(
            "loaded {} unique location ranges and {} unique ASN ranges",
            location_ranges_by_country.len(),
            asn_ranges_by_asn.len()
        );

        Self {
            location_ranges: IpLocationRanges {
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
    ) -> Result<Arc<IpLocationRangeByIp>, AppError> {
        let Some(ranges) = self.location_ranges.by_continent.get(continent) else {
            return Ok(Arc::new(IpLocationRangeByIp::default()));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_country(
        &self,
        country_alpha2: &str,
    ) -> Result<Arc<IpLocationRangeByIp>, AppError> {
        let Some(ranges) = self.location_ranges.by_country.get(country_alpha2) else {
            return Ok(Arc::new(IpLocationRangeByIp::default()));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_asn(&self, asn: &u32) -> Result<Arc<IpAsnRangeByIp>, AppError> {
        let Some(ranges) = self.asn_ranges.by_asn.get(asn) else {
            return Ok(Arc::new(IpAsnRangeByIp::default()));
        };
        Ok(ranges.clone())
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
