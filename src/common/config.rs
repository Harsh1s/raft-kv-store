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
