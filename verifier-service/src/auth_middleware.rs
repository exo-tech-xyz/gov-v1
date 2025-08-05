//! Authentication middleware for upload endpoints

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use tracing::info;

use crate::state::AppState;

/// Middleware that validates token exists and hasn't expired.
pub async fn auth_middleware(
    State(app_state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Only apply auth to upload endpoint
    if request.uri().path() != "/upload" {
        return Ok(next.run(request).await);
    }

    info!("Auth middleware: Validating token for upload request");

    // Extract token from Authorization header
    let token = extract_bearer_token(&headers)?;

    // Check if token exists and hasn't expired
    if !app_state.token_store.is_token_valid(token) {
        info!("Invalid or expired token: {}", token);
        return Err(StatusCode::UNAUTHORIZED);
    }

    info!("Auth middleware: Token is valid, proceeding to handler");

    // Continue to the handler (which will consume the token)
    Ok(next.run(request).await)
}

/// Extract Bearer token from Authorization header
pub fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, StatusCode> {
    let auth_header = headers.get("authorization")
        .ok_or(StatusCode::UNAUTHORIZED)?
        .to_str()
        .map_err(|_| StatusCode::BAD_REQUEST)?;
    
    auth_header.strip_prefix("Bearer ")
        .ok_or(StatusCode::BAD_REQUEST)
}
