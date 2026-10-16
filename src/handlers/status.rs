use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};

use crate::{AppState, error::AppError};

/// `GET /api/status` — returns the per-component health snapshot with the
/// `overall` field filled in from the worst current status.
pub async fn get_status(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let status = state.status.read().await.clone();
    let overall = status.worst();
    Ok(Json(crate::status::AppStatus { overall, ..status }))
}
