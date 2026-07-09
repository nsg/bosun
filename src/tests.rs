//! Integration tests for Bosun's own behavior: default-deny access control and
//! reverse-proxy passthrough. A mock upstream stands in for Frigate so we can
//! assert which requests get forwarded.

use std::net::SocketAddr;

use axum::{Router, extract::Request};

use crate::build_app;
use crate::config::Config;

/// Mock "Frigate": echoes `METHOD path` in the body so tests can prove a
/// request was actually forwarded.
async fn upstream_echo(req: Request) -> String {
    format!("{} {}", req.method(), req.uri().path())
}

async fn spawn(app: Router) -> SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    addr
}

/// Start a mock upstream + a Bosun instance pointed at it, returning Bosun's
/// base URL. The config grants a "viewer" GET/HEAD on events and snapshots, and
/// a "poster" that may only POST to one path.
async fn start_gateway() -> String {
    let upstream = spawn(Router::new().fallback(upstream_echo)).await;

    let toml = format!(
        r#"
listen = "127.0.0.1:0"

[frigate]
url = "http://{upstream}"

[[api_keys]]
name = "viewer"
key = "viewer-key"
  [[api_keys.rules]]
  methods = ["GET", "HEAD"]
  paths = ["/api/events", "/api/*/latest.*"]

[[api_keys]]
name = "poster"
key = "poster-key"
  [[api_keys.rules]]
  methods = ["POST"]
  paths = ["/api/reviews/viewed"]
"#
    );

    let config: Config = toml::from_str(&toml).unwrap();
    let app = build_app(config).unwrap();
    let bosun = spawn(app).await;
    format!("http://{bosun}")
}

fn client() -> reqwest::Client {
    reqwest::Client::new()
}

#[tokio::test]
async fn missing_key_is_unauthorized() {
    let base = start_gateway().await;
    let resp = client()
        .get(format!("{base}/api/events"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn unknown_key_is_unauthorized() {
    let base = start_gateway().await;
    let resp = client()
        .get(format!("{base}/api/events"))
        .header("x-api-key", "nope")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);
}

#[tokio::test]
async fn allowed_request_is_proxied() {
    let base = start_gateway().await;
    let resp = client()
        .get(format!("{base}/api/events"))
        .header("x-api-key", "viewer-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    // Body proves the request reached the mock upstream unchanged.
    assert_eq!(resp.text().await.unwrap(), "GET /api/events");
}

#[tokio::test]
async fn allowed_glob_path_is_proxied() {
    let base = start_gateway().await;
    let resp = client()
        .get(format!("{base}/api/front_door/latest.jpg"))
        .header("x-api-key", "viewer-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "GET /api/front_door/latest.jpg");
}

#[tokio::test]
async fn allowed_head_is_proxied() {
    let base = start_gateway().await;
    let resp = client()
        .head(format!("{base}/api/events"))
        .header("x-api-key", "viewer-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn wrong_verb_is_forbidden() {
    let base = start_gateway().await;
    let resp = client()
        .post(format!("{base}/api/events"))
        .header("x-api-key", "viewer-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn unlisted_path_is_forbidden() {
    let base = start_gateway().await;
    let resp = client()
        .get(format!("{base}/api/config"))
        .header("x-api-key", "viewer-key")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
}

#[tokio::test]
async fn rules_are_scoped_per_key() {
    let base = start_gateway().await;
    // poster may POST to the one allowed path...
    let ok = client()
        .post(format!("{base}/api/reviews/viewed"))
        .header("x-api-key", "poster-key")
        .send()
        .await
        .unwrap();
    assert_eq!(ok.status(), 200);
    // ...but not GET it, and not touch the viewer's paths.
    let denied_verb = client()
        .get(format!("{base}/api/reviews/viewed"))
        .header("x-api-key", "poster-key")
        .send()
        .await
        .unwrap();
    assert_eq!(denied_verb.status(), 403);

    let denied_path = client()
        .get(format!("{base}/api/events"))
        .header("x-api-key", "poster-key")
        .send()
        .await
        .unwrap();
    assert_eq!(denied_path.status(), 403);
}

#[tokio::test]
async fn healthz_needs_no_key() {
    let base = start_gateway().await;
    let resp = client()
        .get(format!("{base}/healthz"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.text().await.unwrap(), "ok");
}
