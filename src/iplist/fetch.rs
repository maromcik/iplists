use std::{
    collections::HashMap,
    time::{Duration, SystemTime},
};

use log::{debug, info};
use time::OffsetDateTime;
use tokio::io::AsyncWriteExt;

use crate::{error::AppError, iplist::parse::Parser};

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
