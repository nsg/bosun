mod config;
mod proxy;

use std::sync::Arc;

use anyhow::Context;
use axum::{Router, routing::get};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use config::Config;
use proxy::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bosun=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "bosun.toml".into());
    let config =
        Config::load(&config_path).with_context(|| format!("loading config from {config_path}"))?;

    let listen = config.listen.clone();
    let upstream = config.frigate.url.clone();

    let client = reqwest::Client::builder()
        .build()
        .context("building HTTP client")?;

    let state = Arc::new(AppState { config, client });

    let app = Router::new()
        .route("/healthz", get(healthz))
        .fallback(proxy::handler)
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding to {listen}"))?;

    tracing::info!("Bosun listening on {listen}, proxying to {upstream}");

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}

async fn healthz() -> &'static str {
    "ok"
}
