use crate::blocklist::config::BlocklistConfig;
use crate::error::AppError;
use crate::iptools::network::{ListNetwork, NetworkType};
use ipnetwork::{Ipv4Network, Ipv6Network};
use log::{debug, error, warn};
use std::fmt::Display;
use std::str::FromStr;
use std::time::Duration;
use tokio::fs::DirEntry;

#[derive(Default, Debug)]
pub struct BlocklistRanges {
    pub ipv4: Vec<NetworkType<Ipv4Network>>,
    pub ipv6: Vec<NetworkType<Ipv6Network>>,
}

impl BlocklistRanges {
    pub async fn merged_blocklist_ranges(config: &BlocklistConfig) -> BlocklistRanges {
        debug!("downloading blocklist");
        let mut merged = BlocklistRanges::default();
        match BlocklistRanges::download(&config.clone()).await {
            Ok(ranges) => merged.merge(ranges),
            Err(e) => {
                error!("failed to download blocklist: {}", e);
            }
        };

        debug!("loading custom ranges");
        match BlocklistRanges::load(&config.clone()).await {
            Ok(ranges) => merged.merge(ranges),
            Err(e) => {
                error!("failed to load custom ranges: {}", e);
            }
        };
        merged
    }

    pub async fn download(config: &BlocklistConfig) -> Result<BlocklistRanges, AppError> {
        let ipv4 = fetch_blocklist(config, &config.ipv4_url).await?;
        let ipv6 = fetch_blocklist(config, &config.ipv6_url).await?;

        let ipv4 = validate_subnets::<Ipv4Network>(&ipv4, None);
        let ipv6 = validate_subnets::<Ipv6Network>(&ipv6, None);
        Ok(BlocklistRanges { ipv4, ipv6 })
    }

    pub async fn load(config: &BlocklistConfig) -> Result<BlocklistRanges, AppError> {
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

async fn read_file<T>(f: DirEntry, ranges: &mut Vec<NetworkType<T>>, config: &BlocklistConfig)
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

async fn fetch_blocklist(
    config: &BlocklistConfig,
    endpoint: &str,
) -> Result<Vec<String>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout))
        .build()?;

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

pub fn validate_subnets<T>(ips: &[String], log: Option<&str>) -> Vec<NetworkType<T>>
where
    T: ListNetwork + FromStr + Display + std::fmt::Debug,
    <T as FromStr>::Err: Display,
    AppError: From<<T as FromStr>::Err>,
{
    let mut parsed = Vec::new();
    for ip in ips {
        match ip.parse::<T>() {
            Ok(parsed_ip) => {
                if parsed_ip.is_network() {
                    parsed.push(NetworkType::Ip(parsed_ip));
                } else {
                    warn!("{}:invalid ip: {ip}; not a network", log.unwrap_or(""));
                }
            }
            Err(e) => {
                let mut ip_split = ip.split("-");
                let start = ip_split.next();
                let end = ip_split.next();
                if let (Some(start), Some(end)) = (start, end)
                    && let (Ok(start), Ok(end)) = (start.parse::<T>(), end.parse::<T>())
                {
                    debug!("parsed range: {start} - {end}");
                    parsed.push(NetworkType::Range(start, end));
                    continue;
                }
                warn!("{}:ip could not be parsed: {ip}; {e}", log.unwrap_or(""));
            }
        }
    }

    parsed
}

pub fn join_ips<T>(ips: &[NetworkType<T>]) -> String
where
    T: ListNetwork + FromStr + Display + std::fmt::Debug,
    <T as FromStr>::Err: Display,
    AppError: From<<T as FromStr>::Err>,
{
    ips.iter()
        .map(|ip| ip.to_string())
        .collect::<Vec<String>>()
        .join("\n")
}
