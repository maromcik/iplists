use axum::{extract::Request, http::header, middleware::Next, response::IntoResponse};
use std::collections::HashMap;
use std::sync::Arc;

use crate::AppState;
use crate::error::AppError;

/// Maps an API token to the username it belongs to.
pub type Users = HashMap<String, String>;

/// Loads users from a plain text file.
///
/// The file format is one user per line:
///
/// ```text
/// # comments and empty lines are ignored
/// alice:super-secret-token
/// bob:another-secret-token
/// ```
///
/// If no `auth_token_file_path` is configured, authentication is disabled and an empty map is returned.
pub async fn load_users(path: &Option<String>) -> Result<Users, AppError> {
    match path {
        None => Ok(Users::new()),
        Some(path) => {
            let content = tokio::fs::read_to_string(path).await?;
            let mut users = Users::new();

            for (line_no, line) in content.lines().enumerate() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                let mut parts = line.splitn(2, ':');
                let username = parts
                    .next()
                    .ok_or_else(|| {
                        AppError::ConfigError(format!("Missing username on line {}", line_no + 1))
                    })?
                    .trim();
                let token = parts
                    .next()
                    .ok_or_else(|| {
                        AppError::ConfigError(format!("Missing token on line {}", line_no + 1))
                    })?
                    .trim();

                if username.is_empty() || token.is_empty() {
                    return Err(AppError::ConfigError(format!(
                        "Empty username or token on line {}",
                        line_no + 1
                    )));
                }

                users.insert(token.to_string(), username.to_string());
            }

            Ok(users)
        }
    }
}

/// Axum middleware that protects API routes with a bearer token.
///
/// The token is read from the `Authorization` header, either as
/// `Authorization: Bearer <token>` or `Authorization: <token>`.
///
/// If no `auth_token_file_path` is configured, all requests are allowed.
pub async fn auth_middleware(
    state: axum::extract::State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<impl IntoResponse, AppError> {
    if state.config.auth_token_file_path.is_none() {
        return Ok(next.run(request).await);
    }

    let token = match request.headers().get(header::AUTHORIZATION) {
        Some(header) => {
            let value = header
                .to_str()
                .map_err(|_| AppError::Unauthorized("Invalid Authorization header".to_string()))?;
            if let Some(bearer) = value.strip_prefix("Bearer ") {
                bearer.to_string()
            } else {
                value.to_string()
            }
        }
        None => {
            return Err(AppError::Unauthorized(
                "Missing Authorization header".to_string(),
            ));
        }
    };

    if state.users.contains_key(&token) {
        Ok(next.run(request).await)
    } else {
        Err(AppError::Unauthorized("Invalid token".to_string()))
    }
}
