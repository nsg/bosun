use std::sync::Arc;

use axum::{
    Json,
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde_json::json;

use crate::proxy::AppState;

const API_KEY_HEADER: &str = "x-api-key";

/// Default-deny access control: authenticate the `X-API-Key`, then permit the
/// request only if one of that key's rules matches its method + path.
pub async fn authorize(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();

    if path == "/healthz" {
        return next.run(req).await;
    }

    let provided = req
        .headers()
        .get(API_KEY_HEADER)
        .and_then(|v| v.to_str().ok());

    let Some(provided) = provided else {
        return error(StatusCode::UNAUTHORIZED, "missing X-API-Key header");
    };

    let Some(key) = state.config.find_key(provided) else {
        return error(StatusCode::UNAUTHORIZED, "invalid API key");
    };

    let method = req.method().clone();
    if !key.permits(method.as_str(), &path) {
        tracing::warn!(key = %key.name, %method, %path, "denied: no matching allow rule");
        return error(
            StatusCode::FORBIDDEN,
            "not permitted: no allow rule matches this method and path",
        );
    }

    tracing::debug!(key = %key.name, %method, %path, "allowed");
    next.run(req).await
}

fn error(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({ "error": msg }))).into_response()
}
