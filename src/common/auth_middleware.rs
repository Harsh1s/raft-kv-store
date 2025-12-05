//! Authentication middleware for axum routes.

use axum::{
    body::Body,
    extract::{Request, State},
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::common::auth::{AuthConfig, AuthContext, AuthResult, KeyStore, KEY_STORE};

#[derive(Clone, Debug)]
pub struct AuthExtension(pub Option<AuthContext>);

#[derive(Clone)]
pub struct AuthState {
    pub key_store: Arc<KeyStore>,
    pub config: AuthConfig,
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            key_store: KEY_STORE.clone(),
            config: AuthConfig::default(),
        }
    }
}

/// Validates the Authorization header and adds AuthContext to request extensions
pub async fn auth_middleware(
    State(state): State<AuthState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    if !state.config.enabled {
        request.extensions_mut().insert(AuthExtension(None));
        return next.run(request).await;
    }

    let path = request.uri().path();
    if state
        .config
        .public_paths
        .iter()
        .any(|p| path.starts_with(p))
    {
        request.extensions_mut().insert(AuthExtension(None));
        return next.run(request).await;
    }

    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let api_key_header = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok());

