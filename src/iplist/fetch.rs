use std::{
    collections::HashMap,
    io::Cursor,
    time::{Duration, SystemTime},
};

use flate2::read::GzDecoder;
use log::info;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::error::AppError;

pub struct Downloader<'a> {
    uri: &'a str,
    timeout: Duration,
    headers: &'a HashMap<String, String>,
}

impl<'a> Downloader<'a> {
    pub fn new(uri: &'a str, timeout: Duration, headers: &'a HashMap<String, String>) -> Self {
        Self {
            uri,
            timeout,
            headers,
        }
    }

    pub async fn download(&self) -> Result<Saver, AppError> {
        let client = reqwest::Client::builder().timeout(self.timeout).build()?;
        let mut req = client.get(self.uri);
        for (k, v) in self.headers {
            req = req.header(k, v);
        }

        let body = req.send().await?.bytes().await?.to_vec();
        info!("data fetched from: {}", self.uri);
        Ok(Saver { body })
    }
}

pub struct Loader {
    pub folder: String,
    pub filename: String,
}

impl Loader {
    pub fn new(folder: &str, filename: &str) -> Self {
        Self {
            folder: folder.to_string(),
            filename: filename.to_string(),
        }
    }

    pub async fn load(&self) -> Result<Parser, AppError> {
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
                .checked_add(Duration::from_hours(24))
                .ok_or(AppError::DataFileLoadError(
                    "Could not increment time to compare downloaded file age".to_string(),
                ))?
        {
            return Err(AppError::DataFileLoadError(
                "Downloaded file is older than 24 hours".to_string(),
            ));
        }
        let body = tokio::fs::read(&path).await?;
        info!("loaded file: {}", path);
        Ok(Parser { body })
    }
}

pub struct Saver {
    body: Vec<u8>,
}

impl Saver {
    pub async fn save(self, folder: &str, filename: &str) -> Result<Parser, AppError> {
        tokio::fs::create_dir_all(format!("{}/download", folder)).await?;
        let path = format!("{}/download/{}", folder, filename);
        let mut file = tokio::fs::File::create(&path).await?;
        file.write_all(&self.body).await?;
        info!("data saved to {path}");
        Ok(Parser { body: self.body })
    }
}

pub struct Parser {
    body: Vec<u8>,
}

impl Parser {
    pub async fn parse<T: Serialize + for<'a> Deserialize<'a>>(&self) -> Result<Vec<T>, AppError> {
        let cursor = Cursor::new(&self.body);
        let decoder = GzDecoder::new(cursor);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_reader(decoder);
        let mut data = Vec::new();
        for record in reader.deserialize() {
            let range: T = record?;
            data.push(range);
        }
        info!("data parsed");
        Ok(data)
    }
}
