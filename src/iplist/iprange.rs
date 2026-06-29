use flate2::read::GzDecoder;
use log::info;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;
use std::{io::Cursor, net::IpAddr};
use tokio::time::Instant;

use crate::iplist::formatter::OutputFormat;
use crate::{error::AppError, iplist::config::GeoConfig};

// pub trait IpRangeTrait: Serialize + for<'de> Deserialize<'de> + Eq + PartialEq {
//     type T: IpRangeTrait;

//     fn download(config: &GeoConfig) -> impl Future<Output = Result<Self::T, AppError>>;
// }
//

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
    pub async fn download(config: &GeoConfig) -> Result<Vec<Self>, AppError> {
        download::<Self>(&config.country_uri, config.timeout, &config.headers).await
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    #[serde(rename = "name")]
    pub country_name: String,
    #[serde(rename = "alpha-2")]
    pub country_alpha2: String,
    #[serde(rename = "region")]
    pub continent: String,
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
    pub async fn parse(config: &GeoConfig) -> Result<Vec<Self>, AppError> {
        let ranges = IpCountryRangeOnly::download(config).await?;
        let locations: Vec<Location> = csv::Reader::from_path(&config.location_path)?
            .deserialize()
            .collect::<Result<Vec<Location>, _>>()?;
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
    pub async fn parse(config: &GeoConfig) -> Result<Vec<IpAsnRange>, AppError> {
        let ranges = Vec::from_iter(
            download::<Self>(&config.asn_uri, config.timeout, &config.headers).await?,
        );

        info!("Parsed {} ASN IP ranges", ranges.len(),);

        Ok(ranges)
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRanges {
    pub all: Arc<Vec<IpLocationRange>>,
    pub by_country: HashMap<String, Arc<Vec<IpLocationRange>>>,
    pub by_continent: HashMap<String, Arc<Vec<IpLocationRange>>>,
}

impl IpLocationRanges {
    pub async fn save(&self, config: &GeoConfig) -> Result<(), AppError> {
        for (country, ranges) in &self.by_country {
            let path = format!("{}/{}", config.output_folder, country);
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
            let path = format!("{}/{}", config.output_folder, continent);
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
    pub asn_ranges: Vec<IpAsnRange>,
}

impl IpRanges {
    pub fn new(location_ranges: Vec<IpLocationRange>, asn_ranges: Vec<IpAsnRange>) -> Self {
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
            asn_ranges,
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

    pub async fn get_by_asn(&self, asn: &u32) -> Result<Vec<IpAsnRange>, AppError> {
        let ranges = self
            .asn_ranges
            .iter()
            .filter(|r| r.asn == *asn)
            .cloned()
            .collect::<Vec<_>>();
        Ok(ranges)
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

impl IpLocationRange {}

pub async fn download<T: Serialize + for<'a> Deserialize<'a>>(
    uri: &str,
    timeout: Duration,
    headers: &HashMap<String, String>,
) -> Result<Vec<T>, AppError> {
    let client = reqwest::Client::builder().timeout(timeout).build()?;
    let mut req = client.get(uri);
    for (k, v) in headers {
        req = req.header(k, v);
    }

    let body = req.send().await?.bytes().await?.to_vec();
    let cursor = Cursor::new(body);
    let decoder = GzDecoder::new(cursor);
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(decoder);
    let mut data = Vec::new();
    for record in reader.deserialize() {
        let range: T = record?;
        data.push(range);
    }
    info!("data fetched from: {}", uri);
    Ok(data)
}
