use std::fmt::Display;

use crate::iplist::iprange::BaseIpRange;
use axum::http::{HeaderValue, header};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

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
        T: Serialize + BaseIpRange,
    {
        match self {
            OutputFormat::Text => FormattedOutput::new(
                data.iter()
                    .map(|ip| format!("{}-{}", ip.start(), ip.end()))
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
                        "set {}_ipv4 {{ \n\ttype ipv4_addr\n\tcounter\n\tflags interval\n\tauto-merge\n\telements = {{\n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                output6.push_str(
                    format!(
                        "set {}_ipv6 {{ \n\ttype ipv6_addr\n\tcounter\n\tflags interval\n\tauto-merge\n\telements = {{\n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                for ip in data {
                    if ip.start().is_ipv4() && ip.end().is_ipv4() {
                        output.push_str(&format!("\t\t{}-{},\n", ip.start(), ip.end()));
                    } else {
                        output6.push_str(&format!("\t\t{}-{},\n", ip.start(), ip.end()));
                    }
                }
                output.push_str("\t}\n}");
                output6.push_str("\t}\n}");
                output.push('\n');
                output.push_str(output6.as_str());
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
