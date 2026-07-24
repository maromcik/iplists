use crate::AppState;
use crate::blocklist::fetch::join_ips;
use crate::error::AppError;
use crate::forms::IpVersion;
use crate::forms::blocklist::BlocklistIpVersion;
use crate::forms::extractors::AppQuery;
use crate::iptools::network::ListNetwork;
use axum::extract::State;
use axum::response::IntoResponse;
use serde::Serialize;
use std::hash::Hash;
use std::sync::Arc;

pub async fn get_blocklist<T>(
    State(state): State<Arc<AppState<T>>>,
    AppQuery(form): AppQuery<BlocklistIpVersion>,
) -> Result<impl IntoResponse, AppError>
where
    T: ListNetwork + Clone + Eq + PartialEq + Serialize + Hash + Send + Sync,
{
    let out: String = match form.version {
        None => {
            let mut res = join_ips(&state.blocklist_ranges.read().await.ipv4);
            res.push('\n');
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
