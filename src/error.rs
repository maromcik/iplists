use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fmt::Debug;
use std::num::ParseIntError;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Serialize)]
pub struct GenericError {
    pub code: u16,
    pub error: AppError,
}

#[derive(Error, Clone, Serialize)]
pub enum AppError {
    #[error("Internal server error: {0}")]
    InternalServerError(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Templating error: {0}")]
    TemplatingError(String),
    #[error("Identity error: {0}")]
    IdentityError(String),
    #[error("Session error: {0}")]
    SessionError(String),
    #[error("Cookie error: {0}")]
    CookieError(String),
    #[error("File error: {0}")]
    FileError(String),
    #[error("Data File load error: {0}")]
    DataFileLoadError(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Could not load the env var: {0}")]
    EnvVarError(String),
    #[error("Config error: {0}")]
    ConfigError(String),
    #[error("Request error: {0}")]
    RequestError(String),
    #[error("Format error: {0}")]
    FmtError(String),
    #[error("Datetime error: {0}")]
    DatetimeError(String),
    #[error("Scheduler error: {0}")]
    SchedulerError(String),
}

impl Debug for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{0}", self)
    }
}

impl From<JoinError> for AppError {
    fn from(value: JoinError) -> Self {
        Self::InternalServerError(value.to_string())
    }
}

impl From<askama::Error> for AppError {
    fn from(error: askama::Error) -> Self {
        Self::TemplatingError(error.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        Self::FileError(value.to_string())
    }
}

impl From<ParseIntError> for AppError {
    fn from(_: ParseIntError) -> Self {
        Self::IdentityError("Invalid User ID".to_string())
    }
}

impl From<std::str::ParseBoolError> for AppError {
    fn from(value: std::str::ParseBoolError) -> Self {
        Self::ParseError(value.to_string())
    }
}

impl From<std::net::AddrParseError> for AppError {
    fn from(value: std::net::AddrParseError) -> Self {
        Self::ParseError(value.to_string())
    }
}

impl From<ipnetwork::IpNetworkError> for AppError {
    fn from(value: ipnetwork::IpNetworkError) -> Self {
        Self::ParseError(value.to_string())
    }
}

impl From<env::VarError> for AppError {
    fn from(value: env::VarError) -> Self {
        Self::EnvVarError(value.to_string())
    }
}

impl From<config::ConfigError> for AppError {
    fn from(value: config::ConfigError) -> Self {
        Self::ConfigError(value.to_string())
    }
}

impl From<reqwest::Error> for AppError {
    fn from(value: reqwest::Error) -> Self {
        let error = value.to_string();
        if let Some(s) = value.source() {
            AppError::RequestError(format!("{}: {}", error, s))
        } else {
            AppError::RequestError(error)
        }
    }
}

impl From<strfmt::FmtError> for AppError {
    fn from(e: strfmt::FmtError) -> Self {
        AppError::FmtError(e.to_string())
    }
}

impl From<csv::Error> for AppError {
    fn from(value: csv::Error) -> Self {
        AppError::ParseError(value.to_string())
    }
}

impl From<time::error::IndeterminateOffset> for AppError {
    fn from(e: time::error::IndeterminateOffset) -> Self {
        AppError::DatetimeError(e.to_string())
    }
}

impl From<tokio_cron_scheduler::JobSchedulerError> for AppError {
    fn from(e: tokio_cron_scheduler::JobSchedulerError) -> Self {
        AppError::SchedulerError(e.to_string())
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status_code = match self {
            AppError::BadRequest(_) | AppError::ParseError(_) => StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Unauthorized(_) => StatusCode::UNAUTHORIZED,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let template = GenericError {
            code: status_code.as_u16(),
            error: self,
        };
        (status_code, Json(template)).into_response()
    }
}
