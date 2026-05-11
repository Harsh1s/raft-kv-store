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

    let auth_result = if let Some(header) = auth_header {
        state.key_store.authenticate(header)
    } else if let Some(key) = api_key_header {
        state.key_store.validate_key(key)
    } else {
        let is_read = matches!(request.method().as_str(), "GET" | "HEAD" | "OPTIONS");
        if is_read && !state.config.require_auth_for_reads {
            request.extensions_mut().insert(AuthExtension(None));
            return next.run(request).await;
        }
        AuthResult::Missing
    };

    match auth_result {
        AuthResult::Ok(ctx) => {
            state.key_store.touch_key(&ctx.key_id);
            request.extensions_mut().insert(AuthExtension(Some(ctx)));
            next.run(request).await
        }
        AuthResult::Missing => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Authentication required",
                "hint": "Provide Authorization header with 'Bearer <jwt>' or 'ApiKey <key>'"
            })),
        )
            .into_response(),
        AuthResult::Invalid(msg) => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Invalid credentials",
                "message": msg
            })),
        )
            .into_response(),
        AuthResult::Expired => (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Credentials expired",
                "hint": "Please generate a new API key or refresh your token"
            })),
        )
            .into_response(),
        AuthResult::Forbidden(msg) => (
            StatusCode::FORBIDDEN,
            Json(json!({
                "error": "Access denied",
                "message": msg
            })),
        )
            .into_response(),
    }
}

pub async fn require_write_middleware(request: Request<Body>, next: Next) -> Response {
    if let Some(AuthExtension(Some(ref ctx))) = request.extensions().get::<AuthExtension>() {
        if !ctx.can_write() {
            return (
                StatusCode::FORBIDDEN,
                Json(json!({
                    "error": "Write permission required",
                    "role": format!("{:?}", ctx.role)
                })),
            )
                .into_response();
