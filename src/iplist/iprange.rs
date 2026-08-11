use crate::iplist::formatter::{OutputFormat, save_data};
use crate::iptools::iptrie::IPTrie;
use crate::iptools::network::ListNetwork;
use crate::status::AppStatus;
use crate::{error::AppError, iplist::config::IplistConfig};
use ipnet::IpNet;
use itertools::Itertools;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Default, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    pub id: String,
    pub name: String,
    pub code: String,
    pub continent: String,
}

#[allow(async_fn_in_trait)]
pub trait LocationParser {
    async fn locations(config: &IplistConfig) -> Result<Vec<Location>, AppError>;

    async fn location_ranges(
        config: &IplistConfig,
        locations: &[Location],
    ) -> Result<Vec<IpLocationRange>, AppError>;
}

#[allow(async_fn_in_trait)]
pub trait AsnParser {
    /// IP subnets with their ASN and ISP.
    async fn asn_ranges(config: &IplistConfig) -> Result<Vec<IpAsnRange>, AppError>;
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRange {
    pub network: IpNet,
    pub location: Location,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRange {
    pub network: IpNet,
    pub asn: u32,
    pub isp: String,
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

#[derive(Debug, Clone, Default)]
pub struct IPTrieLocationRanges {
    pub ipv4: IPTrie<IpLocationRange>,
    pub ipv6: IPTrie<IpLocationRange>,
}

#[derive(Debug, Clone, Default)]
pub struct IPTrieAsnRanges {
    pub ipv4: IPTrie<IpAsnRange>,
    pub ipv6: IPTrie<IpAsnRange>,
}

#[derive(Clone, Default)]
pub struct IpRanges {
    pub location_ranges: IpLocationRanges,
    pub asn_ranges: IpAsnRanges,
    pub trie_location_ranges: IPTrieLocationRanges,
    pub trie_asn_ranges: IPTrieAsnRanges,
    pub locations: Arc<Vec<Location>>,
}

impl IpRanges {
    pub fn new(
        mut location_ranges: Vec<IpLocationRange>,
        mut asn_ranges: Vec<IpAsnRange>,
        locations: Vec<Location>,
    ) -> Self {
        let mut location_ranges_by_country: HashMap<String, IpLocationRangeByIp> = HashMap::new();
        let mut location_ranges_by_continent: HashMap<String, IpLocationRangeByIp> = HashMap::new();
        let mut ipv4_trie_location = IPTrie::new();
        let mut ipv6_trie_location = IPTrie::new();

        location_ranges.sort_by_key(ListNetwork::network_prefix);
        asn_ranges.sort_by_key(ListNetwork::network_prefix);

        for range in location_ranges {
            if range.network.is_ipv4() {
                if ipv4_trie_location.insert(&range) {
                    location_ranges_by_country
                        .entry(range.location.code.clone())
                        .or_default()
                        .ipv4
                        .push(range.clone());
                    location_ranges_by_continent
                        .entry(range.location.continent.clone())
                        .or_default()
                        .ipv4
                        .push(range);
                }
            } else {
                if ipv6_trie_location.insert(&range) {
                    location_ranges_by_country
                        .entry(range.location.code.clone())
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
        }

        let mut ipv4_trie_asn = IPTrie::new();
        let mut ipv6_trie_asn = IPTrie::new();
        let mut asn_ranges_by_asn: HashMap<u32, IpAsnRangeByIp> = HashMap::new();
        for range in &asn_ranges {
            if range.network.is_ipv4() {
                if ipv4_trie_asn.insert(range) {
                    asn_ranges_by_asn
                        .entry(range.asn)
                        .or_default()
                        .ipv4
                        .push(range.clone());
                }
            } else {
                if ipv6_trie_asn.insert(range) {
                    asn_ranges_by_asn
                        .entry(range.asn)
                        .or_default()
                        .ipv6
                        .push(range.clone());
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
            trie_location_ranges: IPTrieLocationRanges {
                ipv4: ipv4_trie_location,
                ipv6: ipv6_trie_location,
            },
            trie_asn_ranges: IPTrieAsnRanges {
                ipv4: ipv4_trie_asn,
                ipv6: ipv6_trie_asn,
            },
            locations: Arc::new(locations),
        }
    }

    pub async fn get_by_continent(
        &self,
        continent: &str,
    ) -> Result<Arc<IpLocationRangeByIp>, AppError> {
        let Some(ranges) = self.location_ranges.by_continent.get(continent) else {
            return Err(AppError::NotFound(format!(
                "No location ranges found for continent: {}",
                continent
            )));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_country(
        &self,
        country_alpha2: &str,
    ) -> Result<Arc<IpLocationRangeByIp>, AppError> {
        let Some(ranges) = self.location_ranges.by_country.get(country_alpha2) else {
            return Err(AppError::NotFound(format!(
                "No location ranges found for country: {}",
                country_alpha2
            )));
        };
        Ok(ranges.clone())
    }

    pub async fn get_by_asn(&self, asn: &u32) -> Result<Arc<IpAsnRangeByIp>, AppError> {
        let Some(ranges) = self.asn_ranges.by_asn.get(asn) else {
            return Err(AppError::NotFound(format!(
                "No ASN ranges found for ASN: {}",
                asn
            )));
        };
        Ok(ranges.clone())
    }
}

pub async fn generate_ranges<P>(config: &IplistConfig, status: &RwLock<AppStatus>) -> IpRanges
where
    P: LocationParser + AsnParser,
{
    let locations = match P::locations(config).await {
        Ok(locations) => locations
            .into_iter()
            .filter(|l| !l.code.is_empty())
            .sorted_by_key(|l| l.code.clone())
            .collect::<Vec<_>>(),
        Err(e) => {
            let mut status = status.write().await;
            let msg = format!("failed to load locations: {e}");
            error!("{msg}");
            status.locations.error(&msg);
            status.geo.warning(&msg);
            vec![]
        }
    };
    let location_ranges = match P::location_ranges(config, &locations).await {
        Ok(ranges) => ranges,
        Err(e) => {
            let mut status = status.write().await;
            let msg = format!("failed to load countries: {e}");
            error!("{msg}");
            status.locations.error(&msg);
            status.geo.warning(&msg);
            vec![]
        }
    };
    let asn_ranges = match P::asn_ranges(config).await {
        Ok(ranges) => ranges,
        Err(e) => {
            let mut status = status.write().await;
            let msg = format!("failed to load ASNs: {e}");
            error!("{msg}");
            status.asns.error(&msg);
            status.geo.warning(&msg);
            vec![]
        }
    };
    let ip_ranges = IpRanges::new(location_ranges, asn_ranges, locations);
    if let Err(e) = ip_ranges.location_ranges.save(config).await {
        let mut status = status.write().await;
        let msg = format!("failed to save locations: {e}");
        error!("{msg}");
        status.locations.warning(msg);
    }

    ip_ranges
}
