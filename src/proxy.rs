use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::config::Config;

pub struct AppState {
    pub config: Config,
    pub client: reqwest::Client,
}

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-authenticate",
    "proxy-authorization",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
];

pub async fn handler(State(state): State<Arc<AppState>>, req: Request) -> Response {
    let method = req.method().clone();
    let path_and_query = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/")
        .to_string();

    let base = state.config.frigate.url.trim_end_matches('/');
    let url = format!("{base}{path_and_query}");

    let mut headers = req.headers().clone();
    headers.remove(header::HOST);
    strip_hop_by_hop(&mut headers);

    let body = reqwest::Body::wrap_stream(req.into_body().into_data_stream());

    let upstream = state
        .client
        .request(method, &url)
        .headers(headers)
        .body(body)
        .send()
        .await;

    let resp = match upstream {
        Ok(resp) => resp,
        Err(err) => {
            tracing::error!(error = %err, %url, "upstream request to Frigate failed");
            return (
                StatusCode::BAD_GATEWAY,
                "upstream request to Frigate failed",
            )
                .into_response();
        }
    };

    let status = resp.status();
    let mut resp_headers = resp.headers().clone();
    strip_hop_by_hop(&mut resp_headers);
    resp_headers.remove(header::CONTENT_LENGTH);

    let mut response = Response::new(Body::from_stream(resp.bytes_stream()));
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;
    response
}

fn strip_hop_by_hop(headers: &mut HeaderMap) {
    for name in HOP_BY_HOP {
        headers.remove(*name);
    }
}
