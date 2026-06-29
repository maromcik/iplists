use std::fmt::Display;

use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

use crate::iplist::iprange::BaseIpRange;

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Text,
    Json,
    Nftables,
}

impl Default for OutputFormat {
    fn default() -> Self {
        Self::Json
    }
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
                        "set {} {{ \n\ttype ipv4_addr\n\tcounter\n\tflags interval\n\tauto-merge\n\telements = {{\n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                output6.push_str(
                    format!(
                        "set {}6 {{ \n\ttype ipv6_addr\n\tcounter\n\tflags interval\n\tauto-merge\n\telements = {{\n",
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
                output.push_str("\n");
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
            OutputFormat::Json => Json(self.output).into_response(),
            OutputFormat::Nftables => self.output.into_response(),
        }
    }
}
