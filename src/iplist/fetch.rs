use std::{
    collections::HashMap,
    io::Cursor,
    time::{Duration, SystemTime},
};

use log::{debug, info};
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

use crate::{error::AppError, iplist::config::BasicAuth};

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
        let body = req.send().await?.bytes().await?.to_vec();
        debug!("data fetched from: {}", uri);
        Ok(Saver { body })
    }
}

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

    pub async fn load(&self) -> Result<GeoData, AppError> {
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
        Ok(GeoData::new(body))
    }
}

pub struct Saver {
    body: Vec<u8>,
}

impl Saver {
    pub async fn save(self, folder: &str, filename: &str) -> Result<GeoData, AppError> {
        tokio::fs::create_dir_all(format!("{}/download", folder)).await?;
        let path = format!("{}/download/{}", folder, filename);
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(&self.body).await?;
        info!("data saved to {path}");
        Ok(GeoData::new(self.body))
    }
}

/// A downloaded geo IP data archive (ZIP bytes). Provider-agnostic; the
/// provider parser (see [`crate::iplist::parse`]) consumes it.
pub struct GeoData {
    body: Vec<u8>,
}

impl GeoData {
    pub fn new(body: Vec<u8>) -> Self {
        Self { body }
    }

    /// Deserializes the rows of the (last) CSV member of the ZIP archive
    /// whose filename ends with `name`.
    pub fn csv_rows<T: for<'de> Deserialize<'de>>(&self, name: &str) -> Result<Vec<T>, AppError> {
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
            let row: T = record?;
            data.push(row);
        }
        debug!("{name} parsed");
        Ok(data)
    }
}
