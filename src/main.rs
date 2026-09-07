use crate::config::AppConfig;
use crate::error::AppError;
use crate::handlers::iplist::{
    geo_location, get_all_continents, get_all_countries, get_by_asn, get_by_location,
};
use crate::handlers::status::get_status;
use crate::iplist::parsers::maxmind::MaxMindParser;
use crate::list::{IpLists, update_ranges};
use crate::status::{AppStatus, ComponentStatus, Schedule};
use crate::utils::request::real_ip_remote_addr;
use axum::extract::{ConnectInfo, Request};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::get;
use axum::{Router, http};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use ipnet::{Ipv4Net, Ipv6Net};
use log::{debug, info};
use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;
use tokio::net::lookup_host;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use tower_http::services::{ServeDir, ServeFile};
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
pub mod list;
pub mod models;
pub mod status;
pub mod utils;

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
    pub ip_lists: IpLists,
    pub blocklist: RwLock<BlocklistRanges<Ipv4Net, Ipv6Net>>,
    pub schedules: Schedule,
}

impl AppState {
    pub async fn new(config: AppConfig) -> Result<Arc<Self>, AppError> {
        let schedules = Schedule::new(&config.blocklist.cron, &config.iplist.cron)?;
        let status = RwLock::new(AppStatus::default());
        let next_iplist_run = schedules.get_next_run_iplist();
        let next_blocklist_run = schedules.get_next_run_blocklist();
        {
            let mut status = status.write().await;
            status.locations = ComponentStatus::new(next_iplist_run);
            status.asns = ComponentStatus::new(next_iplist_run);
            status.geo = ComponentStatus::new(next_iplist_run);
            status.blocklist = ComponentStatus::new(next_blocklist_run);
        }

        let blocklist_ranges =
            BlocklistRanges::merged_blocklist_ranges(&config.blocklist, &status).await;
        Ok(Arc::new(Self {
            config,
            status,
            ip_lists: IpLists::default(),
            blocklist: RwLock::new(blocklist_ranges),
            schedules,
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

async fn access_log(req: Request, next: Next) -> Response {
    let start = Instant::now();

    let remote_addr = real_ip_remote_addr(&req)
        .map(str::to_owned)
        .or_else(|| {
            req.extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|c| c.0.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let method = req.method().clone();
    let uri = req.uri().clone();
    let version = req.version();
    let user_agent = req
        .headers()
        .get(http::header::USER_AGENT)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_owned();
    let referer = req
        .headers()
        .get(http::header::REFERER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-")
        .to_owned();

    let response = next.run(req).await;

    tracing::info!(
        "{remote_addr} \"{method} {uri} {version:?}\" {} \"{referer}\" \"{user_agent}\" {}ms",
        response.status().as_u16(),
        start.elapsed().as_millis(),
    );

    response
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
    update_ranges::<MaxMindParser>(state.clone()).await;
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
        .layer(middleware::from_fn(access_log))
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
    let state_local = state.clone();
    scheduler
        .add(Job::new_async(&config.iplist.cron, move |_uuid, _lock| {
            let state_local = state_local.clone();
            Box::pin(async move {
                debug!("scheduler:starting iplist update");
                update_ranges::<MaxMindParser>(state_local.clone()).await;
                let mut status = state_local.status.write().await;
                let next = state_local.schedules.get_next_run_iplist();

                status.asns.update.update(next);
                status.locations.update.update(next);
                status.geo.update.update(next);
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
                    *state_local.blocklist.write().await = merged;
                    let next = state_local.schedules.get_next_run_blocklist();
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
