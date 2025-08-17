//! Request tracing middleware with unique IDs and timing.

use axum::{
    body::Body,
    extract::ConnectInfo,
    http::{Request, Response},
    middleware::Next,
};
use std::net::SocketAddr;
use std::time::Instant;
use tracing::{info, warn};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "X-Request-ID";

pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

pub async fn request_tracing_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request<Body>,
    next: Next,
) -> Response<Body> {
    let start = Instant::now();

    let request_id = request
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(generate_request_id);

    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();
    let client_ip = addr.ip().to_string();

    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        path = %path,
