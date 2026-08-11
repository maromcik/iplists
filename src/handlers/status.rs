use std::sync::Arc;

use axum::{Json, extract::State, response::IntoResponse};

use crate::{AppState, error::AppError};

pub async fn get_status(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, AppError> {
    let status = state.status.read().await.clone();
    let overall = status.worst();
    Ok(Json(crate::status::AppStatus { overall, ..status }))
}
