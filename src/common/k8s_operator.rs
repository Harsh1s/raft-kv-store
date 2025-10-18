//! Kubernetes operator for MiniKVCluster CRD.

use crate::common::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniKVClusterSpec {
    pub coordinators: CoordinatorSpec,

    pub volumes: VolumeSpec,

    #[serde(default)]
    pub security: SecuritySpec,

    #[serde(default)]
    pub observability: ObservabilitySpec,

    #[serde(default)]
    pub autoscaling: AutoscalingSpec,

    #[serde(default)]
    pub backup: BackupSpec,

    #[serde(default)]
    pub geo: GeoSpec,

    #[serde(default)]
    pub timeseries: TimeseriesSpec,

    #[serde(default)]
    pub tiering: TieringSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoordinatorSpec {
    pub replicas: u32,

    #[serde(default = "default_coordinator_image")]
    pub image: String,

    #[serde(default)]
    pub resources: ResourceRequirements,

    #[serde(default)]
    pub storage: StorageSpec,
}

fn default_coordinator_image() -> String {
    "ghcr.io/whispem/minikv-coord:latest".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeSpec {
    pub replicas: u32,

    #[serde(default = "default_volume_image")]
    pub image: String,

    #[serde(default = "default_replication_factor")]
    pub replication_factor: u32,

    #[serde(default)]
    pub resources: ResourceRequirements,

    #[serde(default)]
    pub storage: StorageSpec,
}

fn default_volume_image() -> String {
    "ghcr.io/whispem/minikv-volume:latest".to_string()
}

fn default_replication_factor() -> u32 {
    3
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirements {
    #[serde(default)]
    pub requests: ResourceList,
    #[serde(default)]
    pub limits: ResourceList,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceList {
    #[serde(default = "default_cpu_request")]
    pub cpu: String,
    #[serde(default = "default_memory_request")]
    pub memory: String,
}

impl Default for ResourceList {
    fn default() -> Self {
        Self {
            cpu: default_cpu_request(),
            memory: default_memory_request(),
        }
    }
}

fn default_cpu_request() -> String {
    "100m".to_string()
}

fn default_memory_request() -> String {
    "256Mi".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSpec {
    #[serde(default = "default_storage_size")]
    pub size: String,
    #[serde(default)]
    pub storage_class_name: String,
}

impl Default for StorageSpec {
    fn default() -> Self {
        Self {
            size: default_storage_size(),
            storage_class_name: String::new(),
        }
    }
}

fn default_storage_size() -> String {
    "10Gi".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecuritySpec {
    #[serde(default)]
    pub tls: TlsSpec,
    #[serde(default)]
    pub authentication: AuthSpec,
    #[serde(default)]
    pub encryption: EncryptionSpec,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub secret_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub admin_secret_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EncryptionSpec {
    #[serde(default)]
    pub at_rest: bool,
    #[serde(default)]
    pub key_secret_name: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ObservabilitySpec {
    #[serde(default)]
    pub metrics: MetricsSpec,
    #[serde(default)]
    pub tracing: TracingSpec,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSpec {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

impl Default for MetricsSpec {
    fn default() -> Self {
        Self {
            enabled: true,
            port: default_metrics_port(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_metrics_port() -> u16 {
    9090
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TracingSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoscalingSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_min_replicas")]
    pub min_replicas: u32,
    #[serde(default = "default_max_replicas")]
    pub max_replicas: u32,
    #[serde(default = "default_cpu_target")]
    pub target_cpu_utilization: u32,
    #[serde(default = "default_memory_target")]
    pub target_memory_utilization: u32,
    #[serde(default = "default_scale_down_stabilization")]
    pub scale_down_stabilization: u32,
}

impl Default for AutoscalingSpec {
    fn default() -> Self {
        Self {
            enabled: false,
            min_replicas: default_min_replicas(),
            max_replicas: default_max_replicas(),
            target_cpu_utilization: default_cpu_target(),
            target_memory_utilization: default_memory_target(),
            scale_down_stabilization: default_scale_down_stabilization(),
        }
    }
}

fn default_min_replicas() -> u32 {
    3
}
fn default_max_replicas() -> u32 {
    10
}
fn default_cpu_target() -> u32 {
    70
}
fn default_memory_target() -> u32 {
    80
}
fn default_scale_down_stabilization() -> u32 {
    300
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackupSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_backup_schedule")]
    pub schedule: String,
    #[serde(default = "default_backup_retention")]
    pub retention: u32,
    #[serde(default)]
    pub destination: BackupDestinationSpec,
}

fn default_backup_schedule() -> String {
    "0 2 * * *".to_string()
}

fn default_backup_retention() -> u32 {
    7
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDestinationSpec {
    #[serde(default = "default_backup_type")]
    pub r#type: String,
    #[serde(default)]
    pub bucket: String,
    #[serde(default)]
    pub secret_name: String,
}

fn default_backup_type() -> String {
    "s3".to_string()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeoSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub region: String,
    #[serde(default)]
    pub zone: String,
    #[serde(default)]
    pub remote_regions: Vec<RemoteRegionSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteRegionSpec {
    pub name: String,
    pub endpoint: String,
    #[serde(default)]
    pub priority: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeseriesSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_retention_days")]
    pub retention_days: u32,
    #[serde(default)]
    pub downsample_rules: Vec<DownsampleRule>,
}

fn default_retention_days() -> u32 {
    30
