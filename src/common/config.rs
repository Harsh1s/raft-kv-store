impl Config {
    /// Loads configuration from a TOML file and overrides with environment variables (prefix MINIKV_)
    pub fn load() -> Self {
        let s = config::Config::builder()
            .add_source(config::File::with_name("config.toml").required(false))
            .add_source(config::File::with_name("config.local.toml").required(false))
            .add_source(config::Environment::with_prefix("MINIKV").separator("_"))
            .build()
            .expect("Failed to load config");
        s.try_deserialize().expect("Failed to parse config")
    }
}

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node_id: String,

    pub role: NodeRole,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub coordinator: Option<CoordinatorConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub volume: Option<VolumeConfig>,

    #[serde(default = "default_log_level")]
    pub log_level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Coordinator,
    Volume,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatorConfig {
    pub bind_addr: SocketAddr,

    pub grpc_addr: SocketAddr,

    pub db_path: PathBuf,

    pub peers: Vec<String>,

    #[serde(default = "default_replicas")]
    pub replicas: usize,

    #[serde(default = "default_election_timeout")]
    pub election_timeout_ms: u64,

    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_ms: u64,

    #[serde(default = "default_snapshot_threshold")]
    pub snapshot_threshold: u64,

    #[serde(default = "default_num_shards")]
    pub num_shards: u64,

    #[serde(default)]
    pub tls_cert_path: Option<String>,

    #[serde(default)]
    pub tls_key_path: Option<String>,
}

fn default_replicas() -> usize {
    3
}
fn default_election_timeout() -> u64 {
    300
}
fn default_heartbeat_interval() -> u64 {
    50
}
fn default_snapshot_threshold() -> u64 {
    10_000
}
fn default_num_shards() -> u64 {
    256
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            bind_addr: "0.0.0.0:8000".parse().unwrap(),
            grpc_addr: "0.0.0.0:8001".parse().unwrap(),
            db_path: PathBuf::from("./coord-data"),
            peers: vec![],
            replicas: default_replicas(),
            election_timeout_ms: default_election_timeout(),
            heartbeat_interval_ms: default_heartbeat_interval(),
            snapshot_threshold: default_snapshot_threshold(),
            num_shards: default_num_shards(),
            tls_cert_path: None,
            tls_key_path: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeConfig {
    pub bind_addr: SocketAddr,

    pub grpc_addr: SocketAddr,

    pub data_path: PathBuf,

    pub wal_path: PathBuf,

    pub coordinators: Vec<String>,

    #[serde(default = "default_max_blob_size")]
    pub max_blob_size: u64,

    #[serde(default = "default_compaction_interval")]
    pub compaction_interval_secs: u64,

    #[serde(default = "default_compaction_threshold")]
    pub compaction_threshold: usize,

    #[serde(default = "default_volume_heartbeat")]
    pub heartbeat_interval_secs: u64,

    #[serde(default = "default_true")]
    pub enable_bloom: bool,

    #[serde(default = "default_true")]
    pub enable_snapshots: bool,

    #[serde(default)]
    pub wal_sync: WalSyncPolicy,
}

fn default_max_blob_size() -> u64 {
    1024 * 1024 * 1024 // 1 GB
}
fn default_compaction_interval() -> u64 {
    300 // 5 minutes
}
fn default_compaction_threshold() -> usize {
    10
}
fn default_volume_heartbeat() -> u64 {
    10
}
fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WalSyncPolicy {
    /// fsync after every write
    #[default]
    Always,
    /// fsync periodically
    Interval,
    Never,
