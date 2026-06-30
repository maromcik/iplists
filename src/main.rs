use crate::config::AppConfig;
use crate::error::AppError;
use crate::handlers::iplist::{
    get_all_continents, get_all_countries, get_by_asn, get_by_location,
};
use crate::iplist::iprange::{IpRanges, generate_ranges};
use axum::Router;
use axum::routing::get;
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use log::{debug, error, info};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::lookup_host;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use tracing_subscriber::EnvFilter;
use crate::blocklist::fetch::BlocklistRanges;
use crate::handlers::blocklist::get_blocklist;

pub mod blocklist;
pub mod config;
pub mod error;
pub mod forms;
pub mod handlers;
pub mod iplist;
pub mod templates;

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// Optional path to a `YAML or TOML` with configuration.
    #[clap(
        short,
        long,
        value_name = "CONFIG_FILE",
        default_value = "iplists.yaml"
    )]
    config: String,
}

pub struct AppState {
    pub config: AppConfig,
    pub ip_ranges: RwLock<IpRanges>,
    pub blocklist_ranges: RwLock<BlocklistRanges>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Arc<Self>, AppError> {
        let ip_ranges = generate_ranges(&config.iplist).await?;
        let blocklist_ranges = BlocklistRanges::download(&config.blocklist).await?;
        Ok(Arc::new(Self {
            config,
            ip_ranges: RwLock::new(ip_ranges),
            blocklist_ranges: RwLock::new(blocklist_ranges),
        }))
    }
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = Cli::parse();

    let config = AppConfig::parse_config(&cli.config)?;

    let env = EnvFilter::new(
        format!("iplists={},{}", config.app_log_level, config.all_log_level).as_str(),
    );
    debug!("Using config: {:?}", &config);

    let timer = tracing_subscriber::fmt::time::LocalTime::rfc_3339();
    tracing_subscriber::fmt()
        .with_timer(timer)
        .with_target(true)
        .with_env_filter(env)
        .init();

    let state = AppState::new(config.clone()).await?;

    let scheduler = JobScheduler::new().await?;
    let config_local = config.iplist.clone();
    let state_local = state.clone();
    scheduler
        .add(Job::new_async(
            &config.iplist.download_cron,
            move |_uuid, _lock| {
                let config_local = config_local.clone();
                let state_local = state_local.clone();
                Box::pin(async move {
                    info!("Scheduler:downloading iplists");
                    match generate_ranges(&config_local.clone()).await {
                        Ok(ranges) => {
                            *state_local.ip_ranges.write().await = ranges;
                        }
                        Err(e) => {
                            error!("Failed to generate ranges: {}", e);
                        }
                    };
                })
            },
        )?)
        .await?;

    let config_local = config.blocklist.clone();
    let state_local = state.clone();
    scheduler
        .add(Job::new_async(
            &config.blocklist.download_cron,
            move |_uuid, _lock| {
                let config_local = config_local.clone();
                let state_local = state_local.clone();
                Box::pin(async move {
                    info!("Scheduler:downloading blocklist");
                    match BlocklistRanges::download(&config_local.clone()).await {
                        Ok(ranges) => {
                            *state_local.blocklist_ranges.write().await = ranges;
                        }
                        Err(e) => {
                            error!("Failed to download blocklist: {}", e);
                        }
                    };
                })
            },
        )?)
            .await?;

    scheduler.start().await?;

    let app = Router::new()
        .fallback_service(
            ServeDir::new("./frontend/dist").fallback(ServeFile::new("./frontend/dist/index.html")),
        )
        .nest_service("/static", ServeDir::new("static"))
        .route("/iplist/country", get(get_all_countries))
        .route("/iplist/continent", get(get_all_continents))
        .route("/iplist/location", get(get_by_location))
        .route("/iplist/asn", get(get_by_asn))
        .route("/iplist/blocklist", get(get_blocklist))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let tls_config = if let (Some(cert), Some(key)) =
        (config.tls_cert_path.as_ref(), config.tls_key_path.as_ref())
    {
        Some(RustlsConfig::from_pem_file(cert, key).await?)
    } else {
        None
    };
    let hostnames = lookup_hosts(&config.hostnames).await?;
    for hostname in hostnames {
        if let Some(ref tls) = tls_config {
            info!("listening with TLS on {}", hostname);
            axum_server::bind_rustls(hostname, tls.clone())
                .serve(app.clone().into_make_service())
                .await?;
        } else {
            info!("listening on {}", hostname);
            axum_server::bind(hostname)
                .serve(app.clone().into_make_service())
                .await?;
        }
    }

    Ok(())
}

async fn lookup_hosts(hostname_set: &HashSet<String>) -> Result<Vec<SocketAddr>, AppError> {
    let mut hostnames = Vec::default();
    for hostname in hostname_set {
        hostnames.extend(lookup_host(hostname).await?)
    }
    Ok(hostnames)
}
