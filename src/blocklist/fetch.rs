use crate::blocklist::config::{BlocklistConfig, CustomListConfig, UrlBlocklist};
use crate::error::AppError;
use crate::iptools::iptrie::{build, deduplicate};
use crate::iptools::network::ListNetwork;
use ipnet::{Ipv4Net, Ipv6Net};
use log::{debug, error, warn};
use std::fmt::Display;
use std::str::FromStr;
use tokio::fs::DirEntry;

#[derive(Default, Debug)]
pub struct BlocklistRanges {
    pub ipv4: Vec<Ipv4Net>,
    pub ipv6: Vec<Ipv6Net>,
}

impl BlocklistRanges {
    pub fn deduplicate(self) -> BlocklistRanges {
        BlocklistRanges {
            ipv4: deduplicate(self.ipv4),
            ipv6: deduplicate(self.ipv6),
        }
    }

    pub async fn merged_blocklist_ranges(config: &BlocklistConfig) -> BlocklistRanges {
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
        match BlocklistRanges::load_custom_lists(&config.custom_blocklist).await {
            Ok(ranges) => merged.merge(ranges),
            Err(e) => {
                error!("failed to load custom blocklist ranges: {}", e);
            }
        };

        debug!("loading custom allowlist ranges");
        let allowlist = match BlocklistRanges::load_custom_lists(&config.custom_allowlist).await {
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

    pub async fn download(config: &UrlBlocklist) -> Result<BlocklistRanges, AppError> {
        let ipv4 = if let Some(url) = &config.ipv4_url {
            let ips = fetch_blocklist(config, url).await?;
            validate_subnets::<Ipv4Net>(&ips, None)
        } else {
            Vec::new()
        };
        let ipv6 = if let Some(url) = &config.ipv6_url {
            let ips = fetch_blocklist(config, url).await?;
            validate_subnets::<Ipv6Net>(&ips, None)
        } else {
            Vec::new()
        };

        Ok(BlocklistRanges { ipv4, ipv6 })
    }

    pub async fn load_custom_lists(config: &CustomListConfig) -> Result<BlocklistRanges, AppError> {
        tokio::fs::create_dir_all(&config.ipv4_folder).await?;
        tokio::fs::create_dir_all(&config.ipv6_folder).await?;
        let mut ipv4_files = tokio::fs::read_dir(&config.ipv4_folder).await?;
        let mut ipv6_files = tokio::fs::read_dir(&config.ipv6_folder).await?;
        let mut ipv4_ranges = Vec::new();
        let mut ipv6_ranges = Vec::new();
        while let Ok(Some(f)) = ipv4_files.next_entry().await {
            read_file(f, &mut ipv4_ranges, config).await;
        }

        while let Ok(Some(f)) = ipv6_files.next_entry().await {
            read_file(f, &mut ipv6_ranges, config).await;
        }

        Ok(BlocklistRanges {
            ipv4: ipv4_ranges,
            ipv6: ipv6_ranges,
        })
    }

    pub fn merge(&mut self, other: BlocklistRanges) {
        self.ipv4.extend(other.ipv4);
        self.ipv6.extend(other.ipv6);
    }
}

async fn read_file<T>(f: DirEntry, ranges: &mut Vec<T>, config: &CustomListConfig)
where
    T: ListNetwork + FromStr + Display + std::fmt::Debug,
    <T as FromStr>::Err: Display,
    AppError: From<<T as FromStr>::Err>,
{
    let content = match tokio::fs::read_to_string(f.path()).await {
        Ok(content) => content,
        Err(e) => {
            warn!(
                "custom ranges: file {} could not be open: {e}",
                f.path().display()
            );
            return;
        }
    };
    let parsed = parse_from_string::<&str>(content.as_str(), config.split_string.as_deref());
    let validated = validate_subnets::<T>(
        &parsed,
        Some(format!("custom ranges: file {}", f.path().display()).as_mut_str()),
    );

    ranges.extend(validated);
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

pub fn validate_subnets<T>(ips: &[String], log: Option<&str>) -> Vec<T>
where
    T: ListNetwork + FromStr + Display + std::fmt::Debug,
    <T as FromStr>::Err: Display,
    AppError: From<<T as FromStr>::Err>,
{
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
