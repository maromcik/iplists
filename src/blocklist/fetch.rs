use crate::blocklist::config::{BlocklistConfig, CustomListConfig, UrlBlocklist};
use crate::error::AppError;
use crate::iptools::iptrie::{build, deduplicate};
use crate::iptools::network::{ListNetwork, Splitable};
use log::{debug, error, warn};
use std::fmt::{Debug, Display};
use std::str::FromStr;
use tokio::fs::DirEntry;

pub trait BlockListNet:
    ListNetwork + FromStr<Err: Display> + Display + Splitable<Output = Self>
{
}

impl<T> BlockListNet for T where
    T: ListNetwork + FromStr<Err: Display> + Display + Splitable<Output = T>
{
}

#[derive(Debug)]
pub struct BlocklistRanges<Ipv4, Ipv6>
where
    Ipv4: BlockListNet,
    Ipv6: BlockListNet,
{
    pub ipv4: Vec<Ipv4>,
    pub ipv6: Vec<Ipv6>,
}

impl<Ipv4, Ipv6> Default for BlocklistRanges<Ipv4, Ipv6>
where
    Ipv4: BlockListNet,
    Ipv6: BlockListNet,
{
    fn default() -> Self {
        Self {
            ipv4: Vec::new(),
            ipv6: Vec::new(),
        }
    }
}

impl<Ipv4, Ipv6> BlocklistRanges<Ipv4, Ipv6>
where
    Ipv4: BlockListNet,
    Ipv6: BlockListNet,
{
    pub fn deduplicate(self) -> BlocklistRanges<Ipv4, Ipv6> {
        BlocklistRanges {
            ipv4: deduplicate(self.ipv4),
            ipv6: deduplicate(self.ipv6),
        }
    }

    pub async fn merged_blocklist_ranges(config: &BlocklistConfig) -> BlocklistRanges<Ipv4, Ipv6> {
        debug!("downloading blocklist");
        let mut merged = BlocklistRanges::default();

        for blocklist in &config.url_blocklist {
            match BlocklistRanges::download(blocklist).await {
                Ok(ranges) => merged.merge(ranges),
                Err(e) => {
                    error!("failed to download blocklist from: {}", e);
                }
            }
        }

        debug!("loading custom blocklist ranges");
        match BlocklistRanges::load(&config.custom_blocklist).await {
            Ok(ranges) => merged.merge(ranges),
            Err(e) => {
                error!("failed to load custom blocklist ranges: {}", e);
            }
        };

        debug!("loading custom allowlist ranges");
        let allowlist = match BlocklistRanges::load(&config.custom_allowlist).await {
            Ok(ranges) => ranges,
            Err(e) => {
                error!("failed to load custom allowlist ranges: {}", e);
                warn!("returning blocklist without allowlist");
                return merged.deduplicate();
            }
        };

        let ipv4_allowlist_trie = build(allowlist.ipv4);
        let ipv6_allowlist_trie = build(allowlist.ipv6);
        let mut ipv4_blocklist = Vec::new();
        let mut ipv6_blocklist = Vec::new();

        for ip in merged.ipv4 {
            let Ok(disjoint) = ipv4_allowlist_trie.subtract(&ip) else {
                warn!("failed to subtract IPv4 allowlist from IP: {}", ip);
                continue;
            };
            ipv4_blocklist.extend(disjoint);
        }

        for ip in merged.ipv6 {
            let Ok(disjoint) = ipv6_allowlist_trie.subtract(&ip) else {
                warn!("failed to subtract IPv6 allowlist from IP: {}", ip);
                continue;
            };
            ipv6_blocklist.extend(disjoint);
        }

        let result = BlocklistRanges {
            ipv4: ipv4_blocklist,
            ipv6: ipv6_blocklist,
        };

        result.deduplicate()
    }

    pub async fn download(config: &UrlBlocklist) -> Result<BlocklistRanges<Ipv4, Ipv6>, AppError> {
        let ipv4 = if let Some(url) = &config.ipv4_url {
            let ips = fetch_blocklist(config, url).await?;
            validate_subnets(&ips, None)
        } else {
            Vec::new()
        };
        let ipv6 = if let Some(url) = &config.ipv6_url {
            let ips = fetch_blocklist(config, url).await?;
            validate_subnets(&ips, None)
        } else {
            Vec::new()
        };

        Ok(BlocklistRanges { ipv4, ipv6 })
    }

    pub async fn load(config: &CustomListConfig) -> Result<BlocklistRanges<Ipv4, Ipv6>, AppError> {
        let mut ipv4_ranges = Vec::new();
        let mut ipv6_ranges = Vec::new();
        match load_custom_lists(&config.ipv4_folder, config.split_string.as_deref()).await {
            Ok(r) => ipv4_ranges.extend(r),
            Err(e) => warn!("could not load the IPv4 list: {e}"),
        }
        match load_custom_lists(&config.ipv6_folder, config.split_string.as_deref()).await {
            Ok(r) => ipv6_ranges.extend(r),
            Err(e) => warn!("could not load the IPv6 list: {e}"),
        }

        Ok(BlocklistRanges {
            ipv4: ipv4_ranges,
            ipv6: ipv6_ranges,
        })
    }

    pub fn merge(&mut self, other: BlocklistRanges<Ipv4, Ipv6>) {
        self.ipv4.extend(other.ipv4);
        self.ipv6.extend(other.ipv6);
    }
}

pub async fn load_custom_lists<T: BlockListNet>(
    folder: &str,
    split: Option<&str>,
) -> Result<Vec<T>, AppError> {
    tokio::fs::create_dir_all(folder).await?;
    let mut files = tokio::fs::read_dir(folder).await?;
    let mut ranges = Vec::new();
    while let Ok(Some(f)) = files.next_entry().await {
        match read_file::<T>(f, split).await {
            Ok(r) => ranges.extend(r),
            Err(e) => warn!("{e}"),
        }
    }

    Ok(ranges)
}

async fn read_file<T: BlockListNet>(f: DirEntry, split: Option<&str>) -> Result<Vec<T>, AppError> {
    let content = tokio::fs::read_to_string(f.path()).await.map_err(|e| {
        AppError::FileError(format!(
            "custom ranges: file {} could not be opened: {e}",
            f.path().display()
        ))
    })?;
    let parsed = parse_from_string::<&str>(content.as_str(), split);
    let validated = validate_subnets::<T>(
        &parsed,
        Some(format!("custom ranges: file {}", f.path().display()).as_mut_str()),
    );

    Ok(validated)
}

pub fn validate_subnets<T: BlockListNet>(ips: &[String], log: Option<&str>) -> Vec<T> {
    let mut parsed = Vec::new();
    for ip in ips {
        match ip.parse::<T>() {
            Ok(parsed_ip) => {
                if parsed_ip.is_net() {
                    parsed.push(parsed_ip);
                } else {
                    warn!("{}:invalid ip: {ip}; not a network", log.unwrap_or(""));
                }
            }
            Err(e) => {
                warn!("{}:ip could not be parsed: {ip}; {e}", log.unwrap_or(""));
            }
        }
    }

    parsed
}

pub fn parse_from_string<S: AsRef<str>>(data: S, split_string: Option<&str>) -> Vec<String> {
    match split_string {
        None => data
            .as_ref()
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .collect(),
        Some(split_str) => data
            .as_ref()
            .split(split_str)
            .map(|s| s.trim().to_string())
            .collect(),
    }
}

async fn fetch_blocklist(config: &UrlBlocklist, endpoint: &str) -> Result<Vec<String>, AppError> {
    let client = reqwest::Client::builder().timeout(config.timeout).build()?;

    let mut req = client.get(endpoint);

    if let Some(headers) = &config.headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    let body = req.send().await?.error_for_status()?.text().await?;

    let blocklist = parse_from_string::<&str>(body.trim(), config.split_string.as_deref());

    debug!("blocklist fetched from: {endpoint}");
    Ok(blocklist)
}
