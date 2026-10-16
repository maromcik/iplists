//! Blocklist aggregation: downloads remote IP blocklists, merges them
//! with local custom lists, subtracts the allowlist and deduplicates the
//! result into the compact set of networks served by `/api/blocklist`.

use crate::blocklist::config::{BlocklistConfig, CustomListConfig, UrlBlocklist};
use crate::error::AppError;
use crate::iptools::iptrie::{build, deduplicate};
use crate::iptools::network::{ListNetwork, Splitable};
use crate::status::{AppStatus, Status, store_worst_status};
use log::{debug, error, warn};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::path::Path;
use std::str::FromStr;
use tokio::fs::DirEntry;
use tokio::sync::RwLock;

/// Everything a network type must provide to participate in the blocklist
/// pipeline: parseable from text, displayable for logs, splittable for
/// allowlist subtraction, and JSON-serializable for the disk backup.
///
/// Blanket-implemented, so `Ipv4Net`, `Ipv6Net`, `IpNet`, etc. qualify
/// automatically.
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

/// A pair of network lists, one per IP family.
///
/// Generic over family-specific concrete types (`Ipv4Net`/`Ipv6Net` in the
/// app) so arithmetic keeps family guarantees; the handler converts to
/// `IpNet` only when both families must go out in one response.
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
    /// Collapses overlapping/duplicate entries into the broadest set of
    /// networks using a prefix trie (e.g. a `/16` absorbs shorter `/24`s).
    pub fn deduplicate(self) -> BlocklistRanges<Ipv4, Ipv6> {
        BlocklistRanges {
            ipv4: deduplicate(self.ipv4),
            ipv6: deduplicate(self.ipv6),
        }
    }

    /// Builds the complete merged blocklist for one refresh run.
    ///
    /// Per source, in order:
    /// 1. every remote list is downloaded (a failure falls back to that
    ///    list's on-disk backup JSON; both failing is recorded),
    /// 2. the custom block lists are loaded from disk and merged in,
    /// 3. the allowlist tries are built and each merged network is
    ///    *subtracted* against them (split only as deeply as needed),
    /// 4. the result is deduplicated.
    ///
    /// Missing allowlists abort the subtraction but still return the
    /// merged set, so a broken allowlist never blocks the pipeline. The
    /// worst failure of the run is stored in the `blocklist` component
    /// status.
    pub async fn merged_blocklist_ranges(
        config: &BlocklistConfig,
        status: &RwLock<AppStatus>,
    ) -> BlocklistRanges<Ipv4, Ipv6> {
        debug!("downloading blocklist");
        let mut merged = BlocklistRanges::default();
        let mut failures: Vec<Status> = Vec::new();
        for blocklist in &config.url_blocklist {
            match BlocklistRanges::download(blocklist).await {
                Ok(ranges) => {
                    if let Err(e) = save_blocklist(&ranges, &blocklist.backup_path).await {
                        let msg = format!("Failed to save blocklist to disk: {}", e);
                        warn!("{msg}");
                        failures.push(Status::warning(msg));
                    } else {
                        debug!("saved blocklist to disk: {}", blocklist.backup_path);
                    }
                    merged.merge(ranges);
                }
                Err(e) => {
                    match load_blocklist::<Ipv4, Ipv6>(&blocklist.backup_path).await {
                        Ok(ranges) => {
                            merged.merge(ranges);
                            debug!("loaded blocklist from disk: {}", blocklist.backup_path);
                        }
                        Err(e) => {
                            let msg = format!("Failed to load blocklist from disk: {}", e);
                            warn!("{msg}");
                            failures.push(Status::warning(msg));
                        }
                    }
                    let msg = format!("Failed to download blocklist from: {}", e);
                    error!("{msg}");
                    failures.push(Status::error(msg));
                }
            }
        }

        debug!("loading custom blocklist ranges");
        match BlocklistRanges::load(&config.custom_blocklist).await {
            Ok(ranges) => merged.merge(ranges),
            Err(e) => {
                let msg = format!("Failed to load custom blocklist ranges: {}", e);
                error!("{msg}");
                failures.push(Status::warning(msg));
            }
        };

        debug!("loading custom allowlist ranges");
        let allowlist = match BlocklistRanges::load(&config.custom_allowlist).await {
            Ok(ranges) => ranges,
            Err(e) => {
                let msg = format!("Failed to load custom allowlist ranges: {}", e);
                error!("{msg}");
                warn!("returning blocklist without allowlist");
                failures.push(Status::warning(msg));
                store_worst_status(status, failures).await;
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
        store_worst_status(status, failures).await;
        result.deduplicate()
    }

    /// Downloads one remote list source. Either of `ipv4_url`/`ipv6_url`
    /// may be absent (treated as an empty family).
    pub async fn download(config: &UrlBlocklist) -> Result<BlocklistRanges<Ipv4, Ipv6>, AppError> {
        let ipv4 = download_urls(config, config.ipv4_url.as_ref()).await?;
        let ipv6 = download_urls(config, config.ipv6_url.as_ref()).await?;

        Ok(BlocklistRanges { ipv4, ipv6 })
    }

    /// Loads every file under the custom list's v4/v6 folders. Files that
    /// fail to parse are skipped with a warning rather than failing the
    /// whole load.
    pub async fn load(config: &CustomListConfig) -> Result<BlocklistRanges<Ipv4, Ipv6>, AppError> {
        let ipv4 = load_custom_lists(&config.ipv4_folder, config.split_string.as_deref()).await?;
        let ipv6 = load_custom_lists(&config.ipv6_folder, config.split_string.as_deref()).await?;

        Ok(BlocklistRanges { ipv4, ipv6 })
    }

    /// Appends another set of ranges; overlaps are resolved later by
    /// [`BlocklistRanges::deduplicate`].
    pub fn merge(&mut self, other: BlocklistRanges<Ipv4, Ipv6>) {
        self.ipv4.extend(other.ipv4);
        self.ipv6.extend(other.ipv6);
    }
}

/// Persists a downloaded list as JSON under `backup_path`, used as the
/// fallback when the next fetch fails.
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

/// Reads back a list previously written by [`save_blocklist`].
async fn load_blocklist<Ipv4: BlockListNet, Ipv6: BlockListNet>(
    path: &str,
) -> Result<BlocklistRanges<Ipv4, Ipv6>, AppError> {
    let content = tokio::fs::read_to_string(path).await?;
    let ranges: BlocklistRanges<Ipv4, Ipv6> = serde_json::from_str(&content)?;
    Ok(ranges)
}

/// Downloads and validates a single URL. A set with no valid networks
/// becomes an error only when `error_on_empty` is configured; otherwise it
/// is warned about and treated as empty.
async fn download_urls<T: BlockListNet>(
    config: &UrlBlocklist,
    url: Option<&String>,
) -> Result<Vec<T>, AppError> {
    let Some(url) = url else {
        return Ok(Vec::new());
    };

    let ips = fetch_blocklist(config, url).await?;

    let log_format = format!("URL Blocklist: {}", config.name);
    let err = AppError::EmptyList(log_format.clone());
    let validated = validate_subnets::<T>(&ips, Some(log_format.as_str()));
    match (config.error_on_empty, validated.is_empty()) {
        (true, true) => Err(err),
        (false, true) => {
            warn!("{err}");
            Ok(Vec::new())
        }
        _ => Ok(validated),
    }
}

/// Parses every file in `folder` as a whitespace- (or custom-delimiter-)
/// separated list of CIDRs, skipping unreadable/invalid files.
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

/// Parses a single custom-list file.
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
        Some(format!("custom ranges: file {}", f.path().display()).as_str()),
    );

    Ok(validated)
}

/// Parses raw strings into network values, keeping only entries that are
/// actual networks (address equals the network base, e.g. `10.0.0.0/8`) —
/// host-bits-set inputs are rejected with a warning so a sloppy source
/// cannot silently mean the wrong range.
pub fn validate_subnets<T: BlockListNet>(ips: &[String], log: Option<&str>) -> Vec<T> {
    let mut parsed = Vec::new();
    for ip in ips {
        match ip.parse::<T>() {
            Ok(parsed_ip) => {
                if parsed_ip.is_net() {
                    parsed.push(parsed_ip);
                } else {
                    warn!("{}: invalid ip: {ip}; not a network", log.unwrap_or(""));
                }
            }
            Err(e) => {
                warn!("{}: ip could not be parsed: {ip}; {e}", log.unwrap_or(""));
            }
        }
    }

    parsed
}

/// Splits a raw payload into candidate list entries: on the configured
/// delimiter when set, otherwise on any whitespace.
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

/// Fetches the remote list as plain text (custom headers supported) and
/// splits it into strings. Parsing/validation is left to the caller.
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
