use crate::config::AppConfig;
use crate::error::AppError;
use crate::handlers::iplist::{
    get_all_continents, get_all_countries, get_by_asn, get_by_location, status,
};
use crate::iplist::iprange::{IpAsnRange, IpLocationRange, IpRanges, Location};
use axum::Router;
use axum::routing::get;
use clap::Parser;
use log::{debug, info};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;
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

    let locations = Location::load(&config.geo)?;
    let location_ranges = IpLocationRange::parse(&config.geo, &locations).await?;
    let asn_ranges = IpAsnRange::parse(&config.geo).await?;
    let ip_ranges = IpRanges::new(location_ranges, asn_ranges, locations);
    ip_ranges.location_ranges.save(&config.geo).await?;
    let ip_ranges = RwLock::new(ip_ranges);

    let state = Arc::new(AppState {
        config: config.clone(),
        ip_ranges,
    });

    let app = Router::new()
        .fallback_service(ServeDir::new("./frontend/dist"))
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
