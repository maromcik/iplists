use crate::AppState;
use crate::blocklist::fetch::join_ips;
use crate::error::AppError;
use crate::forms::blocklist::{BlocklistIpVersion, IpVersion};
use crate::forms::extractors::AppQuery;
use axum::extract::State;
use axum::response::IntoResponse;
use std::sync::Arc;

pub async fn get_blocklist(
    State(state): State<Arc<AppState>>,
    AppQuery(form): AppQuery<BlocklistIpVersion>,
) -> Result<impl IntoResponse, AppError> {
    let out: String = match form.version {
        None => {
            let mut res = join_ips(&state.blocklist_ranges.read().await.ipv4);
            res.push_str(&join_ips(&state.blocklist_ranges.read().await.ipv6));
            res
        }
        Some(ver) => match ver {
            IpVersion::Ipv4 => join_ips(&state.blocklist_ranges.read().await.ipv4),
            IpVersion::Ipv6 => join_ips(&state.blocklist_ranges.read().await.ipv6),
        },
    };
    Ok(out)
}
