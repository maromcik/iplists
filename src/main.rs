use crate::config::AppConfig;
use crate::error::AppError;
use crate::handlers::iplist::{
    geo_location, get_all_continents, get_all_countries, get_by_asn, get_by_location,
};
use crate::handlers::status::get_status;
use crate::iplist::iprange::{IpRanges, generate_ranges};
use crate::iplist::parsers::maxmind::MaxMindParser;
use crate::status::{AppStatus, ComponentStatus, next_run};
use axum::extract::{ConnectInfo, MatchedPath};
use axum::http::{Request, Response};
use axum::routing::get;
use axum::{Router, http};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use ipnet::{Ipv4Net, Ipv6Net};
use log::{debug, info};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::lookup_host;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::services::{ServeDir, ServeFile};
use tower_http::trace::TraceLayer;
use tracing::{field, info_span};
use tracing_appender::non_blocking;

use crate::blocklist::fetch::BlocklistRanges;
use crate::handlers::blocklist::get_blocklist;
use tracing_subscriber::EnvFilter;

pub mod blocklist;
pub mod config;
pub mod error;
pub mod forms;
pub mod handlers;
pub mod iplist;
pub mod iptools;
pub mod models;
pub mod status;

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
    pub status: RwLock<AppStatus>,
    pub ip_ranges: RwLock<IpRanges>,
    pub blocklist_ranges: RwLock<BlocklistRanges<Ipv4Net, Ipv6Net>>,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Arc<Self>, AppError> {
        let status = RwLock::new(AppStatus::default());
        let next_iplist_run = next_run(&config.iplist.cron);
        let next_blocklist_run = next_run(&config.blocklist.cron);
        {
            let mut status = status.write().await;
            status.locations = ComponentStatus::ok_new(next_iplist_run);
            status.asns = ComponentStatus::ok_new(next_iplist_run);
            status.geo = ComponentStatus::ok_new(next_iplist_run);
            status.blocklist = ComponentStatus::ok_new(next_blocklist_run);
        }

        let blocklist_ranges =
            BlocklistRanges::merged_blocklist_ranges(&config.blocklist, &status).await;
        let ip_ranges = generate_ranges::<MaxMindParser>(&config.iplist, &status).await;

        Ok(Arc::new(Self {
            config,
            status,
            ip_ranges: RwLock::new(ip_ranges),
            blocklist_ranges: RwLock::new(blocklist_ranges),
        }))
    }
}

fn main() -> Result<(), AppError> {
    let cli = Cli::parse();
    let config = AppConfig::parse_config(&cli.config)?;

    let worker_threads = config.workers.unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(std::num::NonZero::get)
            .unwrap_or(1)
    });

    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_threads)
        .enable_all()
        .build()
        .expect("failed to build tokio runtime")
        .block_on(run(config))
}

async fn run(config: AppConfig) -> Result<(), AppError> {
    let env = EnvFilter::new(
        format!("iplists={},{}", config.app_log_level, config.all_log_level).as_str(),
    );
    debug!("Using config: {:?}", config);

    let timer = tracing_subscriber::fmt::time::LocalTime::rfc_3339();
    let (non_blocking, _non_blocking_guard) = non_blocking(std::io::stdout());
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_timer(timer)
        .with_target(true)
        .with_env_filter(env)
        .init();

    let state: Arc<AppState> = AppState::new(config.clone()).await?;
    schedule_tasks(state.clone(), &config).await?;

    let api_routes = Router::new()
        .route("/iplist/country", get(get_all_countries))
        .route("/iplist/continent", get(get_all_continents))
        .route("/iplist/location", get(get_by_location))
        .route("/iplist/asn", get(get_by_asn))
        .route("/blocklist", get(get_blocklist))
        .route("/iplist/geo", get(geo_location))
        .route("/status", get(get_status))
        .with_state(state.clone());

    let app = Router::new()
        .fallback_service(
            ServeDir::new("./frontend/dist").fallback(ServeFile::new("./frontend/dist/index.html")),
        )
        .nest_service("/lists", ServeDir::new("lists"))
        .nest("/api", api_routes)
        .nest_service("/static", ServeDir::new("static"))
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|req: &Request<_>| {
                    let matched_path = req
                        .extensions()
                        .get::<MatchedPath>()
                        .map(MatchedPath::as_str);

                    let remote_addr = req
                        .extensions()
                        .get::<ConnectInfo<SocketAddr>>()
                        .map(|c| c.0.to_string())
                        .unwrap_or("unknown".to_string());

                    info_span!(
                        "request",
                        remote_addr = ?remote_addr,
                        method = %req.method(),
                        path = matched_path,
                        uri = %req.uri(),
                        version = ?req.version(),
                        user_agent = ?req.headers().get(http::header::USER_AGENT).map(|v| v.to_str().unwrap_or_default()).unwrap_or("unknown"),
                        referer = ?req.headers().get(http::header::REFERER).map(|v| v.to_str().unwrap_or_default()).unwrap_or("unknown"),
                        status = field::Empty,
                        latency_ms = field::Empty,

                    )
                })
                .on_response(
                    |res: &Response<_>, latency: Duration, span: &tracing::Span| {
                        span.record("status", tracing::field::display(res.status()));
                        span.record("latency_ms", latency.as_millis());

                        tracing::info!(parent: span, "request");
                    },
                ),
        )
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
                .serve(
                    app.clone()
                        .into_make_service_with_connect_info::<SocketAddr>(),
                )
                .await?;
        } else {
            info!("listening on {}", hostname);
            axum_server::bind(hostname)
                .serve(
                    app.clone()
                        .into_make_service_with_connect_info::<SocketAddr>(),
                )
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

async fn schedule_tasks(state: Arc<AppState>, config: &AppConfig) -> Result<(), AppError> {
    let scheduler = JobScheduler::new().await?;
    let config_local = config.iplist.clone();
    let state_local = state.clone();
    scheduler
        .add(Job::new_async(&config.iplist.cron, move |_uuid, _lock| {
            let config_local = config_local.clone();
            let state_local = state_local.clone();
            Box::pin(async move {
                debug!("scheduler:starting iplist update");
                let ranges =
                    generate_ranges::<MaxMindParser>(&config_local, &state_local.status).await;
                let next = next_run(&config_local.cron);
                *state_local.ip_ranges.write().await = ranges;
                let mut status = state_local.status.write().await;
                status.asns.update.update(next);
                status.locations.update.update(next);
                status.geo.update.update(next);
                info!("scheduler:iplist update completed");
            })
        })?)
        .await?;

    let config_local = config.blocklist.clone();
    let state_local = state.clone();
    scheduler
        .add(Job::new_async(
            &config.blocklist.cron,
            move |_uuid, _lock| {
                let config_local = config_local.clone();
                let state_local = state_local.clone();
                Box::pin(async move {
                    debug!("scheduler:starting blocklist update");
                    let merged = BlocklistRanges::merged_blocklist_ranges(
                        &config_local,
                        &state_local.status,
                    )
                    .await;
                    *state_local.blocklist_ranges.write().await = merged;
                    let next = next_run(&config_local.cron);
                    let mut status = state_local.status.write().await;
                    status.blocklist.update.update(next);
                    info!("scheduler:blocklist update completed");
                })
            },
        )?)
        .await?;

    scheduler.start().await?;
    Ok(())
}
