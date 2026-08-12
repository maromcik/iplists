use crate::blocklist::config::{BlocklistConfig, CustomListConfig, UrlBlocklist};
use crate::error::AppError;
use crate::iptools::iptrie::{build, deduplicate};
use crate::iptools::network::{ListNetwork, Splitable};
use crate::status::AppStatus;
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::path::Path;
use std::str::FromStr;
use tokio::fs::DirEntry;
use tokio::sync::RwLock;

pub trait BlockListNet:
    ListNetwork
    + FromStr<Err: Display>
    + Display
    + Splitable<Output = Self>
    + Serialize
    + for<'de> Deserialize<'de>
{
}

impl<T> BlockListNet for T where
    T: ListNetwork
        + FromStr<Err: Display>
        + Display
        + Splitable<Output = T>
        + Serialize
        + for<'de> Deserialize<'de>
{
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BlocklistRanges<Ipv4, Ipv6> {
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

    pub async fn merged_blocklist_ranges(
        config: &BlocklistConfig,
        status: &RwLock<AppStatus>,
    ) -> BlocklistRanges<Ipv4, Ipv6> {
        debug!("downloading blocklist");
        let mut merged = BlocklistRanges::default();
        let mut ok = true;
        for blocklist in &config.url_blocklist {
            match BlocklistRanges::download(blocklist).await {
                Ok(ranges) => {
                    if let Err(e) = save_blocklist(&ranges, &blocklist.backup_path).await {
                        let msg = format!("failed to save blocklist to disk: {}", e);
                        warn!("{msg}");
                        status.write().await.blocklist.warning(msg);
                    } else {
                        debug!("saved blocklist to disk: {}", blocklist.backup_path);
                    }
                    merged.merge(ranges);
                }
                Err(e) => {
                    ok = false;
                    match load_blocklist::<Ipv4, Ipv6>(&blocklist.backup_path).await {
                        Ok(ranges) => {
                            merged.merge(ranges);
                            debug!("loaded blocklist from disk: {}", blocklist.backup_path);
                        }
                        Err(e) => {
                            let msg = format!("failed to load blocklist from disk: {}", e);
                            warn!("{msg}");
                            status.write().await.blocklist.warning(msg);
                        }
                    }
                    let msg = format!("failed to download blocklist from: {}", e);
                    error!("{msg}");
                    status.write().await.blocklist.error(msg);
                }
            }
        }

        debug!("loading custom blocklist ranges");
        match BlocklistRanges::load(&config.custom_blocklist).await {
            Ok(ranges) => merged.merge(ranges),
            Err(e) => {
                ok = false;
                let msg = format!("failed to load custom blocklist ranges: {}", e);
                error!("{msg}");
                status.write().await.blocklist.warning(msg);
            }
        };

        debug!("loading custom allowlist ranges");
        let allowlist = match BlocklistRanges::load(&config.custom_allowlist).await {
            Ok(ranges) => ranges,
            Err(e) => {
                let msg = format!("failed to load custom allowlist ranges: {}", e);
                error!("{msg}");
                warn!("returning blocklist without allowlist");
                status.write().await.blocklist.warning(msg);
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
        if ok {
            status
                .write()
                .await
                .blocklist
                .ok("blocklist fetched successfully");
        }
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
        let ipv4 = load_custom_lists(&config.ipv4_folder, config.split_string.as_deref()).await?;
        let ipv6 = load_custom_lists(&config.ipv6_folder, config.split_string.as_deref()).await?;

        Ok(BlocklistRanges { ipv4, ipv6 })
    }

    pub fn merge(&mut self, other: BlocklistRanges<Ipv4, Ipv6>) {
        self.ipv4.extend(other.ipv4);
        self.ipv6.extend(other.ipv6);
    }
}

async fn save_blocklist<Ipv4: BlockListNet, Ipv6: BlockListNet>(
    ranges: &BlocklistRanges<Ipv4, Ipv6>,
    path: &str,
) -> Result<(), AppError> {
    tokio::fs::create_dir_all(
        Path::new(path)
            .parent()
            .ok_or(AppError::FileError(format!("path has no parent: {}", path)))?,
    )
    .await?;
    let json = serde_json::to_string(ranges)?;
    tokio::fs::write(path, json).await?;
    Ok(())
}

async fn load_blocklist<Ipv4: BlockListNet, Ipv6: BlockListNet>(
    path: &str,
) -> Result<BlocklistRanges<Ipv4, Ipv6>, AppError> {
    let content = tokio::fs::read_to_string(path).await?;
    let ranges: BlocklistRanges<Ipv4, Ipv6> = serde_json::from_str(&content)?;
    Ok(ranges)
}

pub async fn load_custom_lists<T: BlockListNet>(
    folder: &str,
    split: Option<&str>,
) -> Result<Vec<T>, AppError> {
    tokio::fs::create_dir_all(folder).await?;
    let mut files = tokio::fs::read_dir(folder).await?;
    let mut ranges = Vec::new();
    while let Ok(Some(f)) = files.next_entry().await {
        let path = f.path();
        let path = path.to_str().unwrap_or_default();
        match read_file::<T>(f, split).await {
            Ok(r) => ranges.extend(r),
            Err(e) => {
                let msg = format!("could not load file {path}: {e}");
                warn!("{msg}");
            }
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
