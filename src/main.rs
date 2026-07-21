use anyhow::Context;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use bosun::build_app;
use bosun::config::Config;

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
        .unwrap_or_else(|| "bosun.json".into());
    let config =
        Config::load(&config_path).with_context(|| format!("loading config from {config_path}"))?;

    let listen = config.listen.clone();
    let upstream = config.frigate.url.clone();
    let key_count = config.api_keys.len();

    let app = build_app(config)?;

    let listener = tokio::net::TcpListener::bind(&listen)
        .await
        .with_context(|| format!("binding to {listen}"))?;

    tracing::info!("Bosun listening on {listen}, proxying to {upstream} ({key_count} API key(s))");

    axum::serve(listener, app).await.context("server error")?;
    Ok(())
}
