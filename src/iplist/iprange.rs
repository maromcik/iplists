use log::{debug, info, warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;
use std::net::IpAddr;
use std::sync::Arc;
use tokio::time::Instant;

use crate::iplist::fetch::{Downloader, Loader};
use crate::iplist::formatter::OutputFormat;
use crate::iptools::network::{ListNetwork, NetworkType};
use crate::{error::AppError, iplist::config::IplistConfig};

pub trait BaseIpRange {
    type Network: ListNetwork + Clone + Debug;

    fn network_type<'a>(&'a self) -> &'a NetworkType<Self::Network>;
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

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub network: NetworkType<T>,
    pub location: Location,
}

impl<T> BaseIpRange for IpLocationRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    type Network = T;

    fn network_type<'a>(&'a self) -> &'a NetworkType<Self::Network> {
        &self.network
    }
}

impl<T> IpLocationRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
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
                let start = T::from_ip_addr(range.start).ok_or_else(|| {
                    AppError::ParseError(format!("unsupported start IP: {}", range.start))
                })?;
                let end = T::from_ip_addr(range.end).ok_or_else(|| {
                    AppError::ParseError(format!("unsupported end IP: {}", range.end))
                })?;
                parsed_ranges.push(IpLocationRange {
                    network: NetworkType::<T>::Range(start, end),
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

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash,
{
    pub network: NetworkType<T>,
    pub asn: u32,
    pub isp: String,
}

impl<T> TryFrom<IpAsnRangeOnly> for IpAsnRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash,
{
    type Error = AppError;

    fn try_from(r: IpAsnRangeOnly) -> Result<Self, Self::Error> {
        let start = T::from_ip_addr(r.start)
            .ok_or_else(|| AppError::ParseError(format!("unsupported start IP: {}", r.start)))?;
        let end = T::from_ip_addr(r.end)
            .ok_or_else(|| AppError::ParseError(format!("unsupported end IP: {}", r.end)))?;
        Ok(IpAsnRange {
            network: NetworkType::Range(start, end),
            asn: r.asn,
            isp: r.isp,
        })
    }
}

impl<T> BaseIpRange for IpAsnRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    type Network = T;

    fn network_type<'a>(&'a self) -> &'a NetworkType<Self::Network> {
        &self.network
    }
}

impl<T> IpAsnRange<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub async fn parse(config: &IplistConfig) -> Result<Vec<IpAsnRange<T>>, AppError> {
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

        let processed_ranges: Vec<IpAsnRange<T>> = ranges
            .into_iter()
            .map(IpAsnRange::try_from)
            .collect::<Result<Vec<_>, _>>()?;

        info!("parsed {} ASN IP ranges", processed_ranges.len(),);

        Ok(processed_ranges)
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRangeByIp<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub ipv4: Vec<IpAsnRange<T>>,
    pub ipv6: Vec<IpAsnRange<T>>,
}

impl<T> Default for IpAsnRangeByIp<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    fn default() -> Self {
        Self {
            ipv4: Vec::new(),
            ipv6: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub by_asn: HashMap<u32, Arc<IpAsnRangeByIp<T>>>,
}

impl<T> Default for IpAsnRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    fn default() -> Self {
        Self {
            by_asn: HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRangeByIp<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub ipv4: Vec<IpLocationRange<T>>,
    pub ipv6: Vec<IpLocationRange<T>>,
}

impl<T> Default for IpLocationRangeByIp<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    fn default() -> Self {
        Self {
            ipv4: Vec::new(),
            ipv6: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub by_country: HashMap<String, Arc<IpLocationRangeByIp<T>>>,
    pub by_continent: HashMap<String, Arc<IpLocationRangeByIp<T>>>,
}

impl<T> Default for IpLocationRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    fn default() -> Self {
        Self {
            by_country: HashMap::new(),
            by_continent: HashMap::new(),
        }
    }
}

pub async fn save_data<T>(
    data: &[T],
    output: OutputFormat,
    path: &str,
    set_name: Option<&str>,
) -> Result<(), AppError>
where
    T: BaseIpRange + Serialize + Clone,
{
    tokio::fs::write(path, output.format(data, set_name).to_string()).await?;
    Ok(())
}

impl<T> IpLocationRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Serialize + Send,
{
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
pub struct IpRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub location_ranges: IpLocationRanges<T>,
    pub asn_ranges: IpAsnRanges<T>,
    pub locations: Arc<Vec<Location>>,
}

impl<T> IpRanges<T>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Hash + Send,
{
    pub fn new(
        location_ranges: Vec<IpLocationRange<T>>,
        asn_ranges: Vec<IpAsnRange<T>>,
        locations: Vec<Location>,
    ) -> Self {
        let mut location_ranges_by_country: HashMap<String, IpLocationRangeByIp<T>> =
            HashMap::new();
        let mut location_ranges_by_continent: HashMap<String, IpLocationRangeByIp<T>> =
            HashMap::new();
        for range in location_ranges {
            if range.network.is_ipv4() {
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
            } else {
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
        }

        let mut asn_ranges_by_asn: HashMap<u32, IpAsnRangeByIp<T>> = HashMap::new();
        for range in &asn_ranges {
            if range.network.is_ipv4() {
                asn_ranges_by_asn
                    .entry(range.asn)
                    .or_default()
                    .ipv4
                    .push(range.clone());
            } else {
                asn_ranges_by_asn
                    .entry(range.asn)
                    .or_default()
                    .ipv6
                    .push(range.clone());
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
    ) -> Result<Arc<IpLocationRangeByIp<T>>, AppError> {
        let Some(ranges) = self.location_ranges.by_continent.get(continent) else {
            return Ok(Arc::new(IpLocationRangeByIp::default()));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_country(
        &self,
        country_alpha2: &str,
    ) -> Result<Arc<IpLocationRangeByIp<T>>, AppError> {
        let Some(ranges) = self.location_ranges.by_country.get(country_alpha2) else {
            return Ok(Arc::new(IpLocationRangeByIp::default()));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_asn(&self, asn: &u32) -> Result<Arc<IpAsnRangeByIp<T>>, AppError> {
        let Some(ranges) = self.asn_ranges.by_asn.get(asn) else {
            return Ok(Arc::new(IpAsnRangeByIp::default()));
        };
        Ok(ranges.clone())
    }
}

pub async fn generate_ranges<T>(config: &IplistConfig) -> Result<IpRanges<T>, AppError>
where
    T: ListNetwork + Clone + Debug + Eq + PartialEq + Serialize + Hash + Send,
{
    let locations = Location::load(config)?;
    let location_ranges = IpLocationRange::parse(config, &locations).await?;
    let asn_ranges = IpAsnRange::parse(config).await?;
    let ip_ranges = IpRanges::new(location_ranges, asn_ranges, locations);
    ip_ranges.location_ranges.save(config).await?;
    Ok(ip_ranges)
}
