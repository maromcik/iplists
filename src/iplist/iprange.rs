use crate::iplist::formatter::{OutputFormat, save_data};
use crate::iptools::iptrie::IPTrie;
use crate::iptools::network::ListNetwork;
use crate::{error::AppError, iplist::config::IplistConfig};
use ipnet::IpNet;
use log::{debug, info};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::time::Instant;

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
    // Shared with the ~250-entry locations table: ranges only bump a
    // refcount instead of cloning 4 Strings (×3 stores per range).
    pub location: Arc<Location>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRange {
    pub network: IpNet,
    pub asn: u32,
    pub isp: String,
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRangeByIp {
    // Arc-shared with the ASN lookup tries: one allocation per range.
    pub ipv4: Vec<Arc<IpAsnRange>>,
    pub ipv6: Vec<Arc<IpAsnRange>>,
}

#[derive(Default, Clone)]
pub struct IpAsnRanges {
    pub by_asn: HashMap<u32, Arc<IpAsnRangeByIp>>,
    pub trie: IPTrieAsnRanges,
}

impl IpAsnRanges {
    pub fn new(mut asn_ranges: Vec<IpAsnRange>) -> Self {
        let t = Instant::now();
        let mut ipv4_trie_asn = IPTrie::new();
        let mut ipv6_trie_asn = IPTrie::new();
        let mut asn_ranges_by_asn: HashMap<u32, IpAsnRangeByIp> = HashMap::new();

        asn_ranges.sort_by_key(ListNetwork::network_prefix);

        for range in asn_ranges {
            let range = Arc::new(range);
            if range.network.is_ipv4() {
                if ipv4_trie_asn.insert(&range) {
                    asn_ranges_by_asn
                        .entry(range.asn)
                        .or_default()
                        .ipv4
                        .push(range);
                }
            } else {
                if ipv6_trie_asn.insert(&range) {
                    asn_ranges_by_asn
                        .entry(range.asn)
                        .or_default()
                        .ipv6
                        .push(range);
                }
            }
        }

        let asn_ranges_count = asn_ranges_by_asn.len();

        let trie_asn_ranges = IPTrieAsnRanges {
            ipv4: ipv4_trie_asn,
            ipv6: ipv6_trie_asn,
        };

        info!(
            "loaded {} unique ASN ranges in {}ms",
            asn_ranges_count,
            t.elapsed().as_millis()
        );

        Self {
            by_asn: asn_ranges_by_asn
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
            trie: trie_asn_ranges,
        }
    }
}

#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRangeByIp {
    // Arc-shared with the location lookup tries: one allocation per range.
    pub ipv4: Vec<Arc<IpLocationRange>>,
    pub ipv6: Vec<Arc<IpLocationRange>>,
}

#[derive(Default, Clone)]
pub struct IpLocationRanges {
    pub by_country: HashMap<String, Arc<IpLocationRangeByIp>>,
    pub by_continent: HashMap<String, Arc<IpLocationRangeByIp>>,
    pub trie: IPTrieLocationRanges,
    pub locations: Arc<Vec<Location>>,
}

impl IpLocationRanges {
    pub fn new(locations: Vec<Location>, mut location_ranges: Vec<IpLocationRange>) -> Self {
        let t = Instant::now();

        let mut location_ranges_by_country: HashMap<String, IpLocationRangeByIp> = HashMap::new();
        let mut location_ranges_by_continent: HashMap<String, IpLocationRangeByIp> = HashMap::new();
        let mut ipv4_trie_location = IPTrie::new();
        let mut ipv6_trie_location = IPTrie::new();

        location_ranges.sort_by_key(ListNetwork::network_prefix);

        for range in location_ranges {
            let range = Arc::new(range);
            if range.network.is_ipv4() {
                if ipv4_trie_location.insert(&range) {
                    location_ranges_by_country
                        .entry(range.location.code.clone())
                        .or_default()
                        .ipv4
                        .push(Arc::clone(&range));
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
                        .push(Arc::clone(&range));
                    location_ranges_by_continent
                        .entry(range.location.continent.clone())
                        .or_default()
                        .ipv6
                        .push(range);
                }
            }
        }

        let location_ranges_count = location_ranges_by_country.len();

        let trie_location_ranges = IPTrieLocationRanges {
            ipv4: ipv4_trie_location,
            ipv6: ipv6_trie_location,
        };

        info!(
            "loaded {} unique location ranges in {}ms",
            location_ranges_count,
            t.elapsed().as_millis()
        );

        Self {
            by_country: location_ranges_by_country
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
            by_continent: location_ranges_by_continent
                .into_iter()
                .map(|(k, v)| (k, Arc::new(v)))
                .collect(),
            trie: trie_location_ranges,
            locations: Arc::new(locations),
        }
    }

    pub async fn save(&self, config: &IplistConfig) -> Result<(), AppError> {
        let t = Instant::now();
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
        info!(
            "saved country and continent files in {}ms",
            t.elapsed().as_millis()
        );
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct IPTrieLocationRanges {
    pub ipv4: IPTrie<Arc<IpLocationRange>>,
    pub ipv6: IPTrie<Arc<IpLocationRange>>,
}

#[derive(Debug, Clone, Default)]
pub struct IPTrieAsnRanges {
    pub ipv4: IPTrie<Arc<IpAsnRange>>,
    pub ipv6: IPTrie<Arc<IpAsnRange>>,
}
