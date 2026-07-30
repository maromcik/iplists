use std::time::Instant;
use std::{collections::HashMap, io::Cursor};

use ipnet::IpNet;
use log::{debug, info, warn};
use serde::{Deserialize, Serialize};

use crate::{
    error::AppError,
    iplist::{
        config::IplistConfig,
        fetch::{Downloader, Loader},
        iprange::{IpAsnRange, IpLocationRange},
    },
};

pub struct Parser {
    pub body: Vec<u8>,
}

impl Parser {
    pub async fn parse<T: Serialize + for<'a> Deserialize<'a>>(
        &self,
        name: &str,
    ) -> Result<Vec<T>, AppError> {
        let cursor = Cursor::new(&self.body);
        let mut archive = zip::ZipArchive::new(cursor)?;
        debug!(
            "Filenames in archive {}, looking for {}",
            archive.len(),
            name
        );

        let mut filename = String::new();

        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            debug!("{}", file.name());
            if file.name().ends_with(name) {
                debug!("Found! {}", file.name());
                filename = file.name().to_string();
            }
        }
        let file = archive.by_name(&filename)?;
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file);
        let mut data = Vec::new();
        for record in reader.deserialize() {
            let range: T = record?;
            data.push(range);
        }
        debug!("{name} parsed");
        Ok(data)
    }
}
#[derive(Default, Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct Location {
    #[serde(rename(deserialize = "geoname_id"))]
    pub id: String,
    #[serde(rename(deserialize = "country_name"))]
    pub name: String,
    #[serde(rename(deserialize = "country_iso_code"))]
    pub code: String,
    #[serde(rename(deserialize = "continent_name"))]
    pub continent: String,
}

impl Location {
    pub async fn parse(config: &IplistConfig) -> Result<Vec<Self>, AppError> {
        let filename = "ip-country.zip";
        let parser = match Loader::new(&config.output_folder, filename, config.max_age)
            .load()
            .await
        {
            Ok(parser) => parser,
            Err(AppError::DataFileLoadError(e)) => {
                warn!("re-downloading file; cause: {}", e);
                Downloader::new(
                    &config.country_uri,
                    config.timeout,
                    &config.headers,
                    config.basic_auth.as_ref(),
                )
                .download()
                .await?
                .save(&config.output_folder, filename)
                .await?
            }
            Err(e) => {
                return Err(e);
            }
        };

        let locations = parser.parse("GeoLite2-Country-Locations-en.csv").await?;
        Ok(locations)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Hash)]
pub struct IpLocationRangeOnly {
    #[serde(rename(deserialize = "geoname_id"))]
    pub id: String,
    pub network: IpNet,
}

impl IpLocationRangeOnly {
    pub async fn download(config: &IplistConfig) -> Result<Vec<Self>, AppError> {
        let filename = "ip-country.zip";
        let parser = Loader::new(&config.output_folder, filename, config.max_age)
            .load()
            .await?;

        let mut subnets = parser.parse("GeoLite2-Country-Blocks-IPv4.csv").await?;
        let rest = parser.parse("GeoLite2-Country-Blocks-IPv6.csv").await?;
        subnets.extend(rest);
        Ok(subnets)
    }

    pub async fn parse(
        config: &IplistConfig,
        locations: &Vec<Location>,
    ) -> Result<Vec<IpLocationRange>, AppError> {
        let ranges = IpLocationRangeOnly::download(config).await?;
        info!("parsing {} Location IP ranges", ranges.len());
        let t = Instant::now();
        let mut location_map = HashMap::new();
        for location in locations {
            location_map.insert(location.id.clone(), location);
        }
        let mut parsed_ranges = Vec::new();
        for range in ranges {
            if let Some(location) = location_map.get(&range.id) {
                parsed_ranges.push(IpLocationRange {
                    network: range.network,
                    location: (*location).to_owned(),
                });
            }
        }
        info!(
            "parsed {} Location IP subnets in {:?}",
            parsed_ranges.len(),
            t.elapsed()
        );

        Ok(parsed_ranges)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct IpAsnRangeOnly {
    pub network: IpNet,
    #[serde(rename(deserialize = "autonomous_system_number"))]
    pub asn: u32,
    #[serde(rename(deserialize = "autonomous_system_organization"))]
    pub isp: String,
}

impl IpAsnRangeOnly {
    pub async fn parse(config: &IplistConfig) -> Result<Vec<IpAsnRange>, AppError> {
        let filename = "ip-asn.csv.gz";
        let parser = match Loader::new(&config.output_folder, filename, config.max_age)
            .load()
            .await
        {
            Ok(parser) => parser,
            Err(AppError::DataFileLoadError(e)) => {
                warn!("re-downloading file; cause: {}", e);
                Downloader::new(
                    &config.asn_uri,
                    config.timeout,
                    &config.headers,
                    config.basic_auth.as_ref(),
                )
                .download()
                .await?
                .save(&config.output_folder, filename)
                .await?
            }
            Err(e) => {
                return Err(e);
            }
        };

        let mut ranges: Vec<IpAsnRangeOnly> = parser.parse("GeoLite2-ASN-Blocks-IPv4.csv").await?;
        let rest = parser.parse("GeoLite2-ASN-Blocks-IPv6.csv").await?;
        ranges.extend(rest);

        info!("parsing {} ASN IP ranges", ranges.len());

        let t = Instant::now();
        let mut parsed_ranges = Vec::new();
        for range in ranges {
            parsed_ranges.push(IpAsnRange {
                network: range.network,
                asn: range.asn,
                isp: range.isp.clone(),
            });
        }

        info!(
            "parsed {} ASN IP subnets in {:?}",
            parsed_ranges.len(),
            t.elapsed()
        );

        Ok(parsed_ranges)
    }
}
