//! Structured audit logging for admin and sensitive actions.

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    AuthSuccess,
    AuthFailure,
    ApiKeyCreated,
    ApiKeyRevoked,
    ApiKeyDeleted,
    RoleChanged,
    DataPut,
    DataDelete,
    ConfigChanged,
    QuotaExceeded,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
    pub event: AuditEventType,
    pub actor: String,          // user/key id or system
    pub target: Option<String>, // affected resource/key
    pub message: String,
    pub meta: Option<serde_json::Value>,
