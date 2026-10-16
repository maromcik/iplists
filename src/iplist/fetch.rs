use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use log::{debug, info};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

use crate::{error::AppError, iplist::config::BasicAuth};

/// Decodes a raw provider archive/collection into typed CSV rows.
///
/// Implemented by a small wrapper around the downloaded bytes (e.g.
/// [`crate::iplist::parsers::maxmind::MaxMindGeoData`]). `Loader`/`Saver`
/// construct it via [`GeoDataParser::new`], so both the download path and
/// the disk-cache path return a ready-to-parse source.
pub trait GeoDataParser {
    fn new(body: Vec<u8>) -> Self;

    /// Deserializes the file named `name` inside the archive into a vector
    /// of CSV rows.
    fn parse<T: for<'de> Deserialize<'de>>(
        &self,
        name: &str,
    ) -> impl Future<Output = Result<Vec<T>, AppError>>;
}

/// HTTP client for provider archives, built from [`IplistConfig`](crate::iplist::config::IplistConfig)
/// fields.
pub struct Downloader<'a> {
    uri: &'a str,
    timeout: Duration,
    headers: &'a HashMap<String, String>,
    basic_auth: Option<&'a BasicAuth>,
}

impl<'a> Downloader<'a> {
    pub fn new(
        uri: &'a str,
        timeout: Duration,
        headers: &'a HashMap<String, String>,
        basic_auth: Option<&'a BasicAuth>,
    ) -> Self {
        Self {
            uri,
            timeout,
            headers,
            basic_auth,
        }
    }

    /// Downloads the archive and returns an unsaved [`Saver`].
    ///
    /// The URI may contain `{year}`/`{month}`/`{day}` placeholders that are
    /// filled with the *current local date* before the request is issued —
    /// useful for providers whose URLs are date-stamped. Custom headers and
    /// basic auth from the config are applied; non-2xx responses error.
    pub async fn download(&self) -> Result<Saver, AppError> {
        let client = reqwest::Client::builder().timeout(self.timeout).build()?;
        let time = OffsetDateTime::now_local()?;
        let uri = strfmt::strfmt!(self.uri,
            year => format!("{:04}", time.year()),
            month => format!("{:02}", u8::from(time.month())),
            day => format!("{:02}", time.day()))?;
        debug!("downloading from: {}", uri);
        let mut req = client.get(&uri);
        for (k, v) in self.headers {
            req = req.header(k, v);
        }
        if let Some(auth) = &self.basic_auth {
            req = req.basic_auth(&auth.username, Some(&auth.password));
        }
        let body = req
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await?
            .to_vec();
        debug!("data fetched from: {}", uri);
        Ok(Saver { body })
    }
}

/// Reads back a previously downloaded archive from the disk cache
/// (`<folder>/download/<filename>`), enforcing a freshness limit.
pub struct Loader {
    pub folder: String,
    pub filename: String,
    pub max_age: std::time::Duration,
}

impl Loader {
    pub fn new(folder: &str, filename: &str, max_age: std::time::Duration) -> Self {
        Self {
            folder: folder.to_string(),
            filename: filename.to_string(),
            max_age,
        }
    }

    /// Opens the cached file and decodes it as a [`GeoDataParser`] `T`.
    ///
    /// Fails with [`AppError::DataFileLoadError`] when the file is missing,
    /// unreadable, or older than `max_age` — callers (e.g.
    /// `load_or_download` in the parsers) treat that as "re-download".
    pub async fn load<T: GeoDataParser>(&self) -> Result<T, AppError> {
        let path = format!("{}/download/{}", self.folder, self.filename);
        let file = tokio::fs::File::open(&path)
            .await
            .map_err(|e| AppError::DataFileLoadError(e.to_string()))?;
        let metadata = file
            .metadata()
            .await
            .map_err(|e| AppError::DataFileLoadError(e.to_string()))?;
        let file_time = metadata
            .modified()
            .map_err(|e| AppError::DataFileLoadError(e.to_string()))?;
        let current = SystemTime::now();
        if current
            > file_time
                .checked_add(self.max_age)
                .ok_or(AppError::DataFileLoadError(
                    "could not increment time to compare downloaded file age".to_string(),
                ))?
        {
            return Err(AppError::DataFileLoadError(format!(
                "downloaded file is older than the max age: {:?}",
                self.max_age
            )));
        }
        let body = tokio::fs::read(&path).await?;
        info!("loaded file: {}", path);
        Ok(T::new(body))
    }
}

/// Holds freshly downloaded bytes so they can be persisted and handed to
/// a parser in one step.
pub struct Saver {
    body: Vec<u8>,
}

impl Saver {
    /// Writes the bytes to `<folder>/download/<filename>` (creating the
    /// directory) and returns the same data as a [`GeoDataParser`] `T`.
    pub async fn save<T: GeoDataParser>(self, folder: &str, filename: &str) -> Result<T, AppError> {
        tokio::fs::create_dir_all(format!("{}/download", folder)).await?;
        let path = format!("{}/download/{}", folder, filename);
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(&self.body).await?;
        info!("data saved to {path}");
        Ok(T::new(self.body))
    }
}
