//! Backup and restore with full/incremental snapshots.

use crate::common::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupType {
    Full,
    Incremental,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackupStatus {
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupManifest {
    pub id: String,

    pub backup_type: BackupType,

    pub status: BackupStatus,

    pub started_at: DateTime<Utc>,

    pub completed_at: Option<DateTime<Utc>>,

    pub size_bytes: u64,

    pub key_count: u64,

    pub checksum: String,

    pub parent_id: Option<String>,

    pub wal_sequence: u64,

    pub data_files: Vec<BackupFile>,

    pub config: BackupConfig,

    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupFile {
    pub path: String,

    pub size: u64,

    pub checksum: String,

    pub encrypted: bool,

    pub compressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    pub destination: BackupDestination,

    #[serde(default = "default_true")]
    pub compress: bool,

    #[serde(default)]
    pub encrypt: bool,

    #[serde(skip_serializing)]
    pub encryption_key: Option<String>,

    #[serde(default = "default_true")]
    pub include_wal: bool,

    #[serde(default = "default_workers")]
    pub parallel_workers: usize,

    #[serde(default = "default_chunk_size")]
    pub chunk_size: usize,
}

fn default_true() -> bool {
    true
}

fn default_workers() -> usize {
    4
}

fn default_chunk_size() -> usize {
    64 * 1024 * 1024 // 64 MB
}

impl Default for BackupConfig {
    fn default() -> Self {
        Self {
            destination: BackupDestination::Local {
                path: "./backups".to_string(),
            },
            compress: true,
            encrypt: false,
            encryption_key: None,
            include_wal: true,
            parallel_workers: 4,
            chunk_size: 64 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum BackupDestination {
    Local {
        path: String,
    },

    S3 {
        bucket: String,
        prefix: Option<String>,
        endpoint: Option<String>,
        region: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreConfig {
    pub backup_id: String,

    pub source: BackupDestination,

    pub target_path: String,

    #[serde(skip_serializing)]
    pub decryption_key: Option<String>,

    pub point_in_time: Option<DateTime<Utc>>,

    #[serde(default = "default_workers")]
    pub parallel_workers: usize,

    #[serde(default = "default_true")]
    pub verify_checksums: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProgress {
    pub id: String,

    pub phase: String,

    pub total_bytes: u64,

    pub processed_bytes: u64,

    pub percent_complete: f64,

    pub eta_seconds: Option<u64>,

    pub rate_bytes_per_sec: u64,

    pub errors: Vec<String>,
}

pub struct BackupManager {
    active_backups: Arc<RwLock<HashMap<String, BackupProgress>>>,

    manifests: Arc<RwLock<Vec<BackupManifest>>>,
