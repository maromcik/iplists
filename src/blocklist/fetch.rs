use std::fmt::Display;
use crate::blocklist::config::BlocklistConfig;
use crate::error::AppError;
use log::{info, warn};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::str::FromStr;
use std::time::Duration;
use ipnetwork::{Ipv4Network, Ipv6Network};
use crate::blocklist::network::ListNetwork;

pub struct BlocklistRanges {
    pub ipv4: Vec<Ipv4Network>,
    pub ipv6: Vec<Ipv6Network>,
}

impl BlocklistRanges {
    pub async fn download(config: &BlocklistConfig) -> Result<BlocklistRanges, AppError> {
        let ipv4 = fetch_blocklist(config, &config.ipv4_url).await?;
        let ipv6 = fetch_blocklist(config, &config.ipv6_url).await?;

        let ipv4 = validate_subnets::<Ipv4Network>(&ipv4, true)?;
        let ipv6 = validate_subnets::<Ipv6Network>(&ipv6, true)?;
        Ok(BlocklistRanges {
            ipv4,
            ipv6,
        })
    }
}

async fn fetch_blocklist(config: &BlocklistConfig, endpoint: &str) -> Result<Vec<String>, AppError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout))
        .build()?;

    let mut req = client.get(endpoint);

    if let Some(headers) = &config.headers {
        for (k, v) in headers {
            req = req.header(k, v);
        }
    }

    let body = req.send().await?.text().await?;

    let blocklist = parse_from_string::<&str>(body.trim().as_ref(), config.split_string.as_deref());

    info!("blocklist fetched from: {endpoint}");
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

pub fn validate_subnets<T>(ips: &[String], strict: bool) -> Result<Vec<T>, AppError>
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
                    parsed.push(parsed_ip);
                } else if strict {
                    return Err(AppError::ParseError(format!(
                        "invalid ip: {parsed_ip}; not a network"
                    )));
                } else {
                    warn!("invalid ip: {ip}; not a network");
                }
            }
            Err(e) => {
                if strict {
                    return Err(AppError::ParseError(format!("{e}: {ip}")));
                }
                warn!("ip could not be parsed: {ip}; {e}");
            }
        }
    }
    info!("parsed {} ip ranges", parsed.len());
    Ok(parsed)
}

pub fn join_ips<T>(ips: &[T]) -> String
where
    T: ListNetwork + FromStr + Display + std::fmt::Debug,
    <T as FromStr>::Err: Display,
    AppError: From<<T as FromStr>::Err>,
{
    ips.iter().map(|ip| ip.to_string()).collect::<Vec<String>>().join("\n")
}
