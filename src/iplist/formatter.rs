//! Output rendering for network lists.
//!
//! [`OutputFormat`] is a serde-friendly enum accepted as the `format` query
//! parameter on every data endpoint. [`OutputFormat::format`] renders a
//! slice of networks into a [`FormattedOutput`], which is both `Display`
//! (for file saving via [`save_data`]) and an axum `IntoResponse` (for
//! direct HTTP responses, with the right `Content-Type`).
//!
//! **Adding a new format** (see also the README):
//! 1. add a variant to [`OutputFormat`],
//! 2. render the data in the `match` inside [`OutputFormat::format`],
//! 3. set the correct `Content-Type` in the `IntoResponse` impl for it.

use crate::error::AppError;
use crate::iptools::network::ListNetwork;
use axum::http::{HeaderValue, header};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::fmt::Display;

/// Supported serializations of a network list.
///
/// Accepted as the `format` query parameter (case-sensitive, e.g.
/// `?format=Nftables`; JSON is the default) and used internally when
/// persisting generated list files to disk.
#[derive(Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
pub enum OutputFormat {
    /// JSON array of the ranges (serialized via serde).
    #[default]
    Json,
    /// One CIDR per line.
    Text,
    /// nftables `define` blocks: `{name}_ipv4 = { ... }` /
    /// `{name}_ipv6 = { ... }`. Families with no entries are omitted.
    Nftables,
    /// nftables named-set declarations (`type ipv4_addr`/`ipv6_addr` with
    /// `counter`, `flags interval` and `auto-merge`) containing the
    /// networks as elements. Families with no entries are omitted.
    NftablesNamedSets,
}

impl OutputFormat {
    /// Renders `data` in this format. `set_name` names the nftables
    /// defines/sets and defaults to `"list"` when `None`; it is lowercased
    /// and suffixed with `_ipv4`/`_ipv6`.
    ///
    /// `Text` and the nftables formats only need [`ListNetwork`]; `Json`
    /// additionally serializes the whole item, so the output shape depends
    /// on the item type (plain network for blocklists, annotated ranges for
    /// geo/ASN lists).
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
                        "\ndefine {}_ipv4 = {{ \n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                output6.push_str(
                    format!(
                        "\ndefine {}_ipv6 = {{ \n",
                        set_name.unwrap_or("list").to_lowercase()
                    )
                    .as_str(),
                );
                let mut ipv4: bool = false;
                let mut ipv6: bool = false;
                for ip in data {
                    if ip.is_ipv4() {
                        output.push_str(&format!("\t{},\n", ip.network_string()));
                        ipv4 = true;
                    } else {
                        output6.push_str(&format!("\t{},\n", ip.network_string()));
                        ipv6 = true;
                    }
                }
                output.push_str("}\n\n");
                output6.push_str("}\n\n");
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
            OutputFormat::NftablesNamedSets => {
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

/// A rendered network list together with the format it was rendered in,
/// so the HTTP layer can pick the right `Content-Type`.
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
    /// JSON responses get `Content-Type: application/json`; the plain-text
    /// and nftables formats fall back to axum's default `text/plain`.
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
            OutputFormat::NftablesNamedSets => self.output.into_response(),
        }
    }
}

/// Renders `data` with `output` and writes it to `path`.
///
/// Used by [`IpLocationRanges::save`](crate::iplist::iprange::IpLocationRanges::save)
/// to generate the per-country/continent files under `/lists/gen/`.
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
