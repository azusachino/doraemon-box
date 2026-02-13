use axum::{
    extract::State,
    http::{HeaderMap, Request},
    middleware::Next,
    response::Response,
};

use crate::error::ApiError;
use crate::AppState;

pub async fn api_key_auth(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request<axum::body::Body>,
    next: Next<axum::body::Body>,
) -> Result<Response, ApiError> {
    let Some(expected_api_key) = state.api_key.as_deref() else {
        return Ok(next.run(request).await);
    };

    let bearer_key = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let header_key = headers
        .get("x-api-key")
        .and_then(|value| value.to_str().ok());
    let provided = bearer_key.or(header_key);

    match provided {
        Some(value) if value == expected_api_key => Ok(next.run(request).await),
        _ => Err(ApiError::Unauthorized),
    }
}
