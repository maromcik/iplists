use crate::config::AppConfig;
use crate::error::AppError;
use crate::handlers::iplist::{
    get_all_continents, get_all_countries, get_by_asn, get_by_location, status,
};
use crate::iplist::iprange::{IpRanges, generate_ranges};
use axum::Router;
use axum::routing::get;
use clap::Parser;
use log::{debug, error, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;

use tracing_subscriber::EnvFilter;

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
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Arc<Self>, AppError> {
        let ip_ranges = generate_ranges(&config.iplist).await?;
        Ok(Arc::new(Self {
            config,
            ip_ranges: RwLock::new(ip_ranges),
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
                    info!("Running periodic task");
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

    scheduler.start().await?;

    let app = Router::new()
        .fallback_service(
            ServeDir::new("./frontend/dist")
                .fallback(ServeFile::new("./frontend/dist/index.html")),
        )
        .nest_service("/static", ServeDir::new("static"))
        .route("/iplist/country", get(get_all_countries))
        .route("/iplist/continent", get(get_all_continents))
        .route("/iplist/location", get(get_by_location))
        .route("/iplist/asn", get(get_by_asn))
        .route("/status", get(status))
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    for hostname in config.hostnames {
        let listener = tokio::net::TcpListener::bind(hostname).await?;
        info!("listening on {}", listener.local_addr()?);
        axum::serve(listener, app.clone()).await?;
    }

    Ok(())
}
