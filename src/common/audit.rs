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
}

pub struct AuditLogger {
    file: Option<Mutex<File>>,
    to_stdout: bool,
}

pub static AUDIT_LOGGER: Lazy<AuditLogger> = Lazy::new(|| AuditLogger::new("audit.log", true));

impl AuditLogger {
    pub fn new(path: &str, to_stdout: bool) -> Self {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(Mutex::new);
        Self { file, to_stdout }
    }

    pub fn log(&self, entry: AuditEntry) {
        let line = serde_json::to_string(&entry).unwrap_or_else(|_| "{}".to_string());
        if let Some(file) = &self.file {
            if let Ok(mut f) = file.lock() {
                let _ = writeln!(f, "{}", line);
            }
        }
        if self.to_stdout {
            println!("[AUDIT] {}", line);
        }
    }

    pub fn log_event(
