//! Custom axum extractors that deserialize the same payloads as the stock
//! `Json`/`Query`/`Form`/`Path` extractors, but convert rejections into the
//! crate-wide [`AppError`] so API error responses stay uniform.

use axum::{
    Form,
    extract::{FromRequest, FromRequestParts, Json, Path, Query, Request},
    http::request::Parts,
};

use crate::error::AppError;

/// JSON body extractor; deserialization errors surface as 400 `ParseError`.
pub struct AppJson<T>(pub T);

impl<S, T> FromRequest<S> for AppJson<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(err) => Err(AppError::ParseError(err.to_string())),
        }
    }
}

/// Query-string extractor; deserialization errors surface as 400 `ParseError`.
/// This is the extractor all GET data endpoints use.
pub struct AppQuery<T>(pub T);

impl<S, T> FromRequest<S> for AppQuery<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Query::<T>::from_request(req, state).await {
            Ok(Query(value)) => Ok(AppQuery(value)),
            Err(err) => Err(AppError::ParseError(err.to_string())),
        }
    }
}

/// `application/x-www-form-urlencoded` body extractor.
pub struct AppForm<T>(pub T);

impl<S, T> FromRequest<S> for AppForm<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Form::<T>::from_request(req, state).await {
            Ok(Form(value)) => Ok(AppForm(value)),
            Err(err) => Err(AppError::ParseError(err.to_string())),
        }
    }
}

/// Path-parameter extractor.
pub struct AppPath<T>(pub T);

impl<S, T> FromRequestParts<S> for AppPath<T>
where
    S: Send + Sync,
    T: serde::de::DeserializeOwned + Send,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match Path::<T>::from_request_parts(parts, state).await {
            Ok(Path(value)) => Ok(AppPath(value)),
            Err(err) => Err(AppError::ParseError(err.to_string())),
        }
    }
}
