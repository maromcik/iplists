use crate::iplist::formatter::OutputFormat;
use crate::iplist::parse::{IpAsnRangeOnly, IpLocationRangeOnly, Location};
use crate::iptools::network::{ListNetwork, NetworkType};
use crate::{error::AppError, iplist::config::IplistConfig};
use ipnetwork::IpNetwork;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;

pub trait BaseIpRange {
    fn network_type(&self) -> &NetworkType<IpNetwork>;
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRange {
    pub network: NetworkType<IpNetwork>,
    pub location: Location,
}

impl BaseIpRange for IpLocationRange {
    fn network_type(&self) -> &NetworkType<IpNetwork> {
        &self.network
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRange {
    pub network: NetworkType<IpNetwork>,
    pub asn: u32,
    pub isp: String,
}

impl BaseIpRange for IpAsnRange {
    fn network_type(&self) -> &NetworkType<IpNetwork> {
        &self.network
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
    T: BaseIpRange + Serialize + Clone,
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

        let mut asn_ranges_by_asn: HashMap<u32, IpAsnRangeByIp> = HashMap::new();
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
    let location_ranges = IpLocationRangeOnly::parse(config, &locations).await?;
    let asn_ranges = IpAsnRangeOnly::parse(config).await?;
    let ip_ranges = IpRanges::new(location_ranges, asn_ranges, locations);
    ip_ranges.location_ranges.save(config).await?;
    Ok(ip_ranges)
}
