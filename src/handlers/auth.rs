use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use std::sync::Arc;
use crate::AppState;
use std::fs;
use std::collections::HashSet;

pub async fn auth_middleware(
    state: axum::extract::State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    let token_file = match &state.config.auth_token_file_path {
        Some(path) => path,
        None => return next.run(request).await, // No auth configured, allow all
    };

    let auth_header = request.headers().get(header::AUTHORIZATION);

    let token = match auth_header {
        Some(h) => {
            let s = h.to_str().unwrap_or("");
            if s.starts_with("Bearer ") {
                s.replace("Bearer ", "")
            } else {
                s.to_string()
            }
        },
        None => return StatusCode::UNAUTHORIZED.into_response(),
    };

    let allowed_tokens = match fs::read_to_string(token_file) {
        Ok(content) => content.lines().map(|s| s.to_string()).collect::<HashSet<String>>(),
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    if allowed_tokens.contains(&token) {
        next.run(request).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
