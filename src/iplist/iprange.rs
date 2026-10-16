//! Core data model of the geo-IP pipeline: the [`Location`] table, the
//! network ranges annotated with locations or ASNs, the provider-facing
//! [`LocationParser`]/[`AsnParser`] traits, and the in-memory indexes
//! ([`IpLocationRanges`], [`IpAsnRanges`]) built from them.

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

/// A country (or territory) entry from the provider's locations table.
#[derive(Default, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    /// Provider-specific identifier (MaxMind `geoname_id`), used to join
    /// location tables with network blocks.
    pub id: String,
    /// Display name, e.g. `"Germany"`.
    pub name: String,
    /// ISO-3166 alpha-2 code, e.g. `"DE"`. Indexes are keyed by this.
    pub code: String,
    /// Continent name, e.g. `"Europe"`. Indexes are keyed by this.
    pub continent: String,
}

/// Provider-specific source of geographic data. Implement this (plus
/// [`AsnParser`]) to plug in a new data provider; see
/// `src/iplist/parsers/` for details.
#[allow(async_fn_in_trait)]
pub trait LocationParser {
    /// The provider's location table (countries and their IDs).
    async fn locations(config: &IplistConfig) -> Result<Vec<Location>, AppError>;

    /// Networks annotated with the matching entry of `locations`.
    async fn location_ranges(
        config: &IplistConfig,
        locations: &[Location],
    ) -> Result<Vec<IpLocationRange>, AppError>;
}

/// Provider-specific source of ASN data. See [`LocationParser`] for how to
/// plug in a new provider.
#[allow(async_fn_in_trait)]
pub trait AsnParser {
    /// IP subnets with their ASN and ISP.
    async fn asn_ranges(config: &IplistConfig) -> Result<Vec<IpAsnRange>, AppError>;
}

/// A network together with the [`Location`] it belongs to.
///
/// The location is `Arc`-shared with the ~250-entry locations table, so
/// storing it in three indexes (by-country, by-continent, trie) costs only
/// refcount bumps instead of cloning 4 Strings per range.
#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRange {
    pub network: IpNet,
    pub location: Arc<Location>,
}

/// A network together with the announcing ASN and its organization.
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRange {
    pub network: IpNet,
    pub asn: u32,
    pub isp: String,
}

/// Networks of one ASN, split by IP family. Entries are `Arc`-shared with
/// the lookup tries, so each range is allocated exactly once.
#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpAsnRangeByIp {
    pub ipv4: Vec<Arc<IpAsnRange>>,
    pub ipv6: Vec<Arc<IpAsnRange>>,
}

/// In-memory ASN index: a `by_asn` map for "give me all networks of AS N"
/// queries plus longest-prefix-match tries for single-IP lookups.
#[derive(Default, Clone)]
pub struct IpAsnRanges {
    pub by_asn: HashMap<u32, Arc<IpAsnRangeByIp>>,
    pub trie: IPTrieAsnRanges,
}

impl IpAsnRanges {
    /// Builds both indexes from raw provider ranges.
    ///
    /// Ranges are sorted by prefix length first, so the trie (which ignores
    /// networks already covered by a broader stored one) naturally deduplicates:
    /// overlapping or repeated announces collapse to the broadest network,
    /// and only surviving ranges land in the `by_asn` map.
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

/// Networks of one country/continent, split by IP family. Entries are
/// `Arc`-shared with the lookup tries, so each range is allocated once.
#[derive(Default, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct IpLocationRangeByIp {
    pub ipv4: Vec<Arc<IpLocationRange>>,
    pub ipv6: Vec<Arc<IpLocationRange>>,
}

/// In-memory location index: `by_country`/`by_continent` maps for list
/// queries, longest-prefix-match tries for single-IP geo lookups, plus the
/// raw location table backing `/api/iplist/country|continent`.
#[derive(Default, Clone)]
pub struct IpLocationRanges {
    pub by_country: HashMap<String, Arc<IpLocationRangeByIp>>,
    pub by_continent: HashMap<String, Arc<IpLocationRangeByIp>>,
    pub trie: IPTrieLocationRanges,
    pub locations: Arc<Vec<Location>>,
}

impl IpLocationRanges {
    /// Builds all location indexes from the provider data.
    ///
    /// As in [`IpAsnRanges::new`], sorting by prefix length lets the trie
    /// drop networks already covered by a broader one, and only surviving
    /// ranges are inserted into the `by_country`/`by_continent` maps.
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

    /// Writes one file set per country and per continent into
    /// `<output_folder>/gen/`.
    ///
    /// For each entry `X` the following files are produced in both `Text`
    /// and `Nftables` formats: `X.txt`/`X.nft` (v4+v6 merged), `X-ipv4.*`
    /// and `X-ipv6.*`. These files are served statically under `/lists/gen/`.
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

/// Per-family longest-prefix-match tries of location networks,
/// used by `GET /api/iplist/geo`.
#[derive(Debug, Clone, Default)]
pub struct IPTrieLocationRanges {
    pub ipv4: IPTrie<Arc<IpLocationRange>>,
    pub ipv6: IPTrie<Arc<IpLocationRange>>,
}

/// Per-family longest-prefix-match tries of ASN networks,
/// used by `GET /api/iplist/geo`.
#[derive(Debug, Clone, Default)]
pub struct IPTrieAsnRanges {
    pub ipv4: IPTrie<Arc<IpAsnRange>>,
    pub ipv6: IPTrie<Arc<IpAsnRange>>,
}
