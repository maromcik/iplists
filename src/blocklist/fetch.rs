use crate::error::AppError;

async fn fetch_blocklist(endpoint: &str) -> Result<Option<Vec<String>>, AppError> {
    // let client = reqwest::Client::builder().timeout(self.timeout).build()?;
    //
    // let mut req = client.get(endpoint);
    //
    // if let Some(headers) = &self.headers {
    //     for (k, v) in headers {
    //         req = req.header(k, v);
    //     }
    // }
    //
    // let body = req.send().await?.text().await?;
    //
    // let blocklist = parse_from_string(Some(body.trim()).as_ref(), self.split_string.as_deref());
    //
    // info!("blocklist fetched from: {endpoint}");
    Ok(None)
}
