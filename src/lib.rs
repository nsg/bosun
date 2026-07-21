pub mod auth;
pub mod config;
pub mod proxy;

use std::sync::Arc;

use anyhow::Context;
use axum::{Router, middleware, routing::get};

use config::Config;
use proxy::AppState;

/// Build the Bosun router: a `/healthz` route plus a catch-all reverse proxy,
/// gated by the default-deny access-control middleware.
pub fn build_app(config: Config) -> anyhow::Result<Router> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(config.connect_timeout))
        .build()
        .context("building HTTP client")?;
    let state = Arc::new(AppState { config, client });

    Ok(Router::new()
        .route("/healthz", get(healthz))
        .fallback(proxy::handler)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::authorize,
        ))
        .with_state(state))
}

async fn healthz() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests;
