use crate::error::AppError;
use crate::iptools::network::ListNetwork;
use axum::http::{HeaderValue, header};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    #[default]
    Json,
    Text,
    Nftables,
}

impl OutputFormat {
    pub fn format<T>(&self, data: &[T], set_name: Option<&str>) -> FormattedOutput
    where
        T: ListNetwork + Serialize + Clone,
    {
        match self {
            OutputFormat::Text => FormattedOutput::new(
                data.iter()
                    .map(|ip| ip.network_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                OutputFormat::Text,
            ),
            OutputFormat::Json => FormattedOutput::new(
                serde_json::to_string(&data).unwrap_or_default(),
                OutputFormat::Json,
            ),
            OutputFormat::Nftables => {
                let mut output = String::new();
                let mut output6 = String::new();
                output.push_str(
                    format!(
                        "\nset {}_ipv4 {{ \n\ttype ipv4_addr\n\tcounter\n\tflags interval\n\tauto-merge\n\telements = {{\n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                output6.push_str(
                    format!(
                        "\nset {}_ipv6 {{ \n\ttype ipv6_addr\n\tcounter\n\tflags interval\n\tauto-merge\n\telements = {{\n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                let mut ipv4: bool = false;
                let mut ipv6: bool = false;
                for ip in data {
                    if ip.is_ipv4() {
                        output.push_str(&format!("\t\t{},\n", ip.network_string()));
                        ipv4 = true;
                    } else {
                        output6.push_str(&format!("\t\t{},\n", ip.network_string()));
                        ipv6 = true;
                    }
                }
                output.push_str("\t}\n}\n\n");
                output6.push_str("\t}\n}\n\n");
                let output = match (ipv4, ipv6) {
                    (true, true) => {
                        output.push('\n');
                        output.push_str(output6.as_str());
                        output
                    }
                    (true, false) => output,
                    (false, true) => output6,
                    (false, false) => "".to_string(),
                };

                FormattedOutput::new(output, OutputFormat::Nftables)
            }
        }
    }
}

pub struct FormattedOutput {
    pub output: String,
    pub format: OutputFormat,
}

impl FormattedOutput {
    pub fn new(output: String, format: OutputFormat) -> Self {
        Self { output, format }
    }
}

impl Display for FormattedOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.output)
    }
}

impl IntoResponse for FormattedOutput {
    fn into_response(self) -> axum::response::Response {
        match self.format {
            OutputFormat::Text => self.output.into_response(),
            OutputFormat::Json => (
                [(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json"),
                )],
                self.output,
            )
                .into_response(),

            OutputFormat::Nftables => self.output.into_response(),
        }
    }
}

pub async fn save_data<T>(
    data: &[T],
    output: OutputFormat,
    path: &str,
    set_name: Option<&str>,
) -> Result<(), AppError>
where
    T: ListNetwork + Serialize + Clone,
{
    tokio::fs::write(path, output.format(data, set_name).to_string()).await?;
    Ok(())
}
