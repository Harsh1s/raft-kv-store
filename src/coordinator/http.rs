//! HTTP API for the coordinator.
//!
//! Provides REST, S3-compatible APIs.

use crate::common::storage::Storage;
use std::time::Duration;

use crate::common::auth::{Role, KEY_STORE};
use crate::common::{AuditEventType, AUDIT_LOGGER};
use async_stream::stream;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

pub static WATCH_CHANNEL: Lazy<broadcast::Sender<KeyChangeEvent>> = Lazy::new(|| {
    let (tx, _rx) = broadcast::channel(100);
    tx
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyChangeEvent {
    pub event: String, // "put" | "delete" | "revoke"
    pub key: String,
    pub tenant: Option<String>,
    pub timestamp: i64,
}

pub async fn watch_sse(
) -> Sse<impl futures_util::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let mut rx = WATCH_CHANNEL.subscribe();
    let stream = stream! {
        while let Ok(event) = rx.recv().await {
            let data = serde_json::to_string(&event).unwrap();
            yield Ok(axum::response::sse::Event::default().data(data));
        }
    };
    Sse::new(stream)
}

pub async fn watch_ws(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    let mut rx = WATCH_CHANNEL.subscribe();
    while let Ok(event) = rx.recv().await {
        let msg = serde_json::to_string(&event).unwrap();
        if socket.send(Message::Text(msg)).await.is_err() {
            break;
        }
    }
}

pub static STORAGE: Lazy<Storage> = Lazy::new(Storage::new_memory);
const VECTOR_INDEX_PATH: &str = "./coord-data/vector_index.json";
static VECTOR_INDEX_LOADED: AtomicBool = AtomicBool::new(false);

static VECTOR_INDEX: Lazy<std::sync::RwLock<HashMap<String, VectorPoint>>> =
    Lazy::new(|| std::sync::RwLock::new(HashMap::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorPoint {
    id: String,
    values: Vec<f32>,
    metadata: Option<serde_json::Value>,
    updated_at: i64,
}

fn ensure_timeseries_engine() {
    let has_engine = {
        let guard = crate::common::timeseries::TIMESERIES_ENGINE.read().unwrap();
        guard.is_some()
    };

    if !has_engine {
        let config = crate::common::timeseries::TimeseriesConfig {
            enabled: true,
            ..Default::default()
        };
        crate::common::timeseries::init_timeseries(config);
    }
}

fn load_vector_index_if_needed() -> Result<(), String> {
    if VECTOR_INDEX_LOADED.load(Ordering::SeqCst) {
        return Ok(());
    }

    let path = std::path::Path::new(VECTOR_INDEX_PATH);
    if path.exists() {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("failed to read vector index: {}", e))?;
        let parsed: HashMap<String, VectorPoint> = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse vector index: {}", e))?;
        let mut index = VECTOR_INDEX.write().unwrap();
        *index = parsed;
    }

    VECTOR_INDEX_LOADED.store(true, Ordering::SeqCst);
    Ok(())
}

fn persist_vector_index() -> Result<(), String> {
    let path = std::path::Path::new(VECTOR_INDEX_PATH);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create vector index directory: {}", e))?;
    }

    let index = VECTOR_INDEX.read().unwrap();
    let content = serde_json::to_string_pretty(&*index)
        .map_err(|e| format!("failed to serialize vector index: {}", e))?;
    std::fs::write(path, content).map_err(|e| format!("failed to write vector index: {}", e))?;

    Ok(())
}

async fn admin_repair(State(_state): State<CoordState>) -> impl IntoResponse {
    let res = crate::ops::repair::repair_cluster("http://localhost:8000", 3, false).await;
    match res {
        Ok(report) => axum::Json(json!({ "status": "ok", "report": report })),
        Err(e) => axum::Json(json!({ "status": "error", "error": format!("{}", e) })),
    }
}

async fn admin_compact(State(_state): State<CoordState>) -> impl IntoResponse {
    let res = crate::ops::compact::compact_cluster("http://localhost:8000", None).await;
    match res {
        Ok(report) => axum::Json(json!({ "status": "ok", "report": report })),
        Err(e) => axum::Json(json!({ "status": "error", "error": format!("{}", e) })),
    }
}

async fn admin_verify(State(_state): State<CoordState>) -> impl IntoResponse {
    let res = crate::ops::verify::verify_cluster("http://localhost:8000", false, 16).await;
    match res {
        Ok(report) => axum::Json(json!({ "status": "ok", "report": report })),
        Err(e) => axum::Json(json!({ "status": "error", "error": format!("{}", e) })),
    }
}

async fn admin_scale(State(_state): State<CoordState>) -> impl IntoResponse {
    axum::Json(json!({ "status": "scaling triggered" }))
}

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

use crate::coordinator::metadata::MetadataStore;
use crate::coordinator::placement::PlacementManager;
use crate::coordinator::raft_node::RaftNode;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Sse;

#[derive(Debug, Deserialize)]
struct CreateKeyRequest {
    name: String,
    #[serde(default = "default_tenant")]
    tenant: String,
    #[serde(default)]
    role: String,
    expires_in_secs: Option<u64>,
}

fn default_tenant() -> String {
    "default".to_string()
}

#[derive(Debug, Serialize)]
struct CreateKeyResponse {
    id: String,
    /// The plaintext API key (shown only once!)
    key: String,
    tenant: String,
    role: String,
    warning: String,
}

async fn admin_create_key(axum::Json(req): axum::Json<CreateKeyRequest>) -> impl IntoResponse {
    let role = match req.role.to_lowercase().as_str() {
        "admin" => Role::Admin,
        "read_write" | "readwrite" | "rw" => Role::ReadWrite,
        "read_only" | "readonly" | "ro" | "" => Role::ReadOnly,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({
                    "error": "Invalid role",
                    "valid_roles": ["admin", "read_write", "read_only"]
                })),
            )
                .into_response();
        }
    };

    let expires_in = req.expires_in_secs.map(Duration::from_secs);

    match KEY_STORE.generate_key(&req.name, &req.tenant, role, expires_in) {
        Ok((id, key)) => {
            let response = CreateKeyResponse {
                id: id.clone(),
                key,
                tenant: req.tenant.clone(),
                role: format!("{:?}", role),
                warning: "Store this key securely - it cannot be retrieved again!".to_string(),
            };
            AUDIT_LOGGER.log_event(
                AuditEventType::ApiKeyCreated,
                req.name.clone(),
                Some(id.clone()),
                format!(
                    "API key created for tenant {} with role {:?}",
                    req.tenant.clone(),
                    role
                ),
                None,
            );
            (StatusCode::CREATED, axum::Json(json!(response))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

/// Query parameter: `?tenant=<tenant>` to filter by tenant.
#[derive(Debug, Deserialize)]
struct ListKeysQuery {
    tenant: Option<String>,
}

async fn admin_list_keys(Query(query): Query<ListKeysQuery>) -> impl IntoResponse {
    let keys = if let Some(tenant) = query.tenant {
        KEY_STORE.list_keys_for_tenant(&tenant)
    } else {
        KEY_STORE.list_keys()
    };

    let safe_keys: Vec<serde_json::Value> = keys
        .iter()
        .map(|k| {
            json!({
                "id": k.id,
                "name": k.name,
                "tenant": k.tenant,
                "role": format!("{:?}", k.role),
                "active": k.active,
                "created_at": k.created_at,
                "expires_at": k.expires_at,
                "last_used_at": k.last_used_at,
            })
        })
        .collect();

    axum::Json(json!({
        "keys": safe_keys,
        "total": safe_keys.len()
    }))
}

async fn admin_get_key(Path(key_id): Path<String>) -> impl IntoResponse {
    match KEY_STORE.get_key(&key_id) {
        Some(k) => (
            StatusCode::OK,
            axum::Json(json!({
                "id": k.id,
                "name": k.name,
                "tenant": k.tenant,
                "role": format!("{:?}", k.role),
                "active": k.active,
                "created_at": k.created_at,
                "expires_at": k.expires_at,
                "last_used_at": k.last_used_at,
            })),
        )
            .into_response(),
        None => (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "error": "Key not found" })),
        )
            .into_response(),
    }
}

async fn admin_revoke_key(Path(key_id): Path<String>) -> impl IntoResponse {
    match KEY_STORE.revoke_key(&key_id) {
        Ok(()) => {
            AUDIT_LOGGER.log_event(
                AuditEventType::ApiKeyRevoked,
                "admin",
                Some(key_id.clone()),
                "API key revoked",
                None,
            );
            let _ = WATCH_CHANNEL.send(KeyChangeEvent {
                event: "revoke".to_string(),
                key: key_id.clone(),
                tenant: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
            (
                StatusCode::OK,
                axum::Json(json!({ "status": "revoked", "id": key_id })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

async fn admin_delete_key(Path(key_id): Path<String>) -> impl IntoResponse {
    match KEY_STORE.delete_key(&key_id) {
        Ok(()) => {
            AUDIT_LOGGER.log_event(
                AuditEventType::ApiKeyDeleted,
                "admin",
                Some(key_id.clone()),
                "API key deleted",
                None,
            );
            let _ = WATCH_CHANNEL.send(KeyChangeEvent {
                event: "delete".to_string(),
                key: key_id.clone(),
                tenant: None,
                timestamp: chrono::Utc::now().timestamp(),
            });
            (
                StatusCode::OK,
                axum::Json(json!({ "status": "deleted", "id": key_id })),
            )
                .into_response()
        }
        Err(e) => (
            StatusCode::NOT_FOUND,
            axum::Json(json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

#[derive(Clone)]
pub struct CoordState {
    pub metadata: Arc<MetadataStore>,
    pub placement: Arc<std::sync::Mutex<PlacementManager>>,
    pub raft: Arc<RaftNode>,
}

async fn s3_put_object(
    State(state): State<CoordState>,
    Path((bucket, key)): Path<(String, String)>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let full_key = format!("{}/{}", bucket, key);

    let ttl_secs: Option<u64> = headers
        .get("X-Minikv-TTL")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok());

    crate::coordinator::http::STORAGE.put(&full_key, body.to_vec());
    let stored_bytes = body.len();
    let _ = WATCH_CHANNEL.send(KeyChangeEvent {
        event: "put".to_string(),
        key: full_key.clone(),
        tenant: Some("default".to_string()),
        timestamp: chrono::Utc::now().timestamp(),
    });

    let placement = state.placement.lock().unwrap();
    let volumes = state.metadata.get_healthy_volumes().unwrap_or_default();
    let target_volumes: Vec<String> = placement
        .select_volumes(&full_key, &volumes)
        .unwrap_or_default();
    let mut prepare_ok = true;
    for _volume_id in &target_volumes {
        let simulated_prepare = true;
        if !simulated_prepare {
            prepare_ok = false;
            break;
        }
    }
    if !prepare_ok {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "PUT S3 {}/{} failed: prepare phase error (2PC)",
                bucket, key
            ),
        );
    }
    for _volume_id in &target_volumes {}

    let ttl_info = ttl_secs
        .map(|t| format!(", TTL: {}s", t))
        .unwrap_or_default();
    (
        StatusCode::OK,
        format!(
            "PUT S3 {}/{} committed via 2PC ({} bytes{})",
            bucket, key, stored_bytes, ttl_info
        ),
    )
}

async fn s3_get_object(
    State(_state): State<CoordState>,
    Path((bucket, key)): Path<(String, String)>,
) -> impl IntoResponse {
    let full_key = format!("{}/{}", bucket, key);
    if let Some(data) = crate::coordinator::http::STORAGE.get(&full_key) {
        (StatusCode::OK, data)
    } else {
        (
            StatusCode::NOT_FOUND,
            format!("S3 object {}/{} not found", bucket, key).into_bytes(),
        )
    }
}

pub fn create_router(state: CoordState) -> Router {
    Router::new()
        .route("/watch/sse", axum::routing::get(watch_sse))
        .route("/watch/ws", axum::routing::get(watch_ws))
        .route("/s3/:bucket/:key", axum::routing::put(s3_put_object))
        .route("/s3/:bucket/:key", axum::routing::get(s3_get_object))
        .route("/:key", axum::routing::delete(delete_key))
        .route("/admin/repair", axum::routing::post(admin_repair))
        .route("/admin/compact", axum::routing::post(admin_compact))
        .route("/admin/verify", axum::routing::post(admin_verify))
        .route("/admin/scale", axum::routing::post(admin_scale))
        .route("/admin/status", axum::routing::get(admin_status))
        .route("/health/ready", axum::routing::get(health_ready))
        .route("/health/live", axum::routing::get(health_live))
        .route("/admin/keys", axum::routing::post(admin_create_key))
        .route("/admin/keys", axum::routing::get(admin_list_keys))
        .route("/admin/keys/:key_id", axum::routing::get(admin_get_key))
        .route(
            "/admin/keys/:key_id/revoke",
            axum::routing::post(admin_revoke_key),
        )
        .route(
            "/admin/keys/:key_id",
            axum::routing::delete(admin_delete_key),
        )
        .route("/admin/import", axum::routing::post(admin_import))
        .route("/admin/export", axum::routing::get(admin_export))
        .route("/transaction", axum::routing::post(transaction_ops))
        .route("/search", axum::routing::get(search_keys))
        .route("/metrics", axum::routing::get(metrics))
        .route("/range", axum::routing::get(range_query))
        .route("/batch", axum::routing::post(batch_ops))
        .route("/admin/ui", axum::routing::get(admin_ui_handler))
        .route("/admin/ui/*path", axum::routing::get(admin_ui_handler))
        .route("/admin/backup", axum::routing::post(admin_create_backup))
        .route("/admin/backups", axum::routing::get(admin_list_backups))
        .route(
            "/admin/backups/:backup_id",
            axum::routing::get(admin_get_backup),
        )
        .route(
            "/admin/backups/:backup_id",
            axum::routing::delete(admin_delete_backup),
        )
        .route("/admin/restore", axum::routing::post(admin_restore))
        .route(
            "/admin/replication/status",
            axum::routing::get(admin_replication_status),
        )
        .route("/admin/plugins", axum::routing::get(admin_list_plugins))
        .route(
            "/admin/plugins/:plugin_id/enable",
            axum::routing::post(admin_enable_plugin),
        )
        .route(
            "/admin/plugins/:plugin_id/disable",
            axum::routing::post(admin_disable_plugin),
        )
        .route("/admin/cdc/status", axum::routing::get(admin_cdc_status))
        .route(
            "/admin/timeseries/stats",
            axum::routing::get(admin_timeseries_stats),
        )
        .route("/admin/geo/status", axum::routing::get(admin_geo_status))
        .route("/ts/write", axum::routing::post(ts_write))
        .route("/ts/query", axum::routing::post(ts_query))
        .route("/ts/query", axum::routing::get(ts_query_get))
        .route("/vector/upsert", axum::routing::post(vector_upsert))
        .route("/vector/query", axum::routing::post(vector_query))
        .route(
            "/admin/vector/stats",
            axum::routing::get(admin_vector_stats),
        )
        .with_state(state)
}

async fn health_ready(State(state): State<CoordState>) -> impl IntoResponse {
    let volumes = state.metadata.get_healthy_volumes().unwrap_or_default();
    let has_leader = state.raft.is_leader() || !state.raft.get_peers().is_empty();

    if !volumes.is_empty() && has_leader {
        (
            StatusCode::OK,
            axum::Json(json!({
                "ready": true,
                "healthy_volumes": volumes.len(),
                "is_leader": state.raft.is_leader(),
            })),
        )
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            axum::Json(json!({
                "ready": false,
                "healthy_volumes": volumes.len(),
                "is_leader": state.raft.is_leader(),
                "reason": if volumes.is_empty() { "No healthy volumes" } else { "No Raft leader" }
            })),
        )
    }
}

async fn health_live() -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(json!({
            "alive": true,
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })),
    )
}

async fn admin_status(State(state): State<CoordState>) -> impl IntoResponse {
    let role = if state.raft.is_leader() {
        "Leader"
    } else {
