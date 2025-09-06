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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownsampleRule {
    pub after: String,
    pub resolution: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TieringSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub hot_tier: TierConfig,
    #[serde(default)]
    pub warm_tier: TierConfig,
    #[serde(default)]
    pub cold_tier: TierConfig,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TierConfig {
    #[serde(default)]
    pub max_age: String,
    #[serde(default)]
    pub storage_class: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MiniKVClusterStatus {
    pub phase: ClusterPhase,

    #[serde(default)]
    pub conditions: Vec<ClusterCondition>,

    #[serde(default)]
    pub coordinator_status: ComponentStatus,

    #[serde(default)]
    pub volume_status: VolumeStatus,

    #[serde(default)]
    pub endpoints: ClusterEndpoints,

    #[serde(default)]
    pub version: String,

    #[serde(default)]
    pub last_backup: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ClusterPhase {
    #[default]
    Pending,
    Creating,
    Running,
    Updating,
    Failed,
    Deleting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClusterCondition {
    pub r#type: String,
    pub status: String,
    pub last_transition_time: DateTime<Utc>,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub ready: u32,
    pub total: u32,
    #[serde(default)]
    pub leader: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeStatus {
    pub ready: u32,
    pub total: u32,
    #[serde(default)]
    pub total_storage: String,
    #[serde(default)]
    pub used_storage: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClusterEndpoints {
    #[serde(default)]
    pub http: String,
    #[serde(default)]
    pub grpc: String,
    #[serde(default)]
    pub metrics: String,
}

pub struct MiniKVController {
    config: ControllerConfig,

    clusters: Arc<RwLock<BTreeMap<String, MiniKVClusterState>>>,
}

#[derive(Debug, Clone)]
pub struct ControllerConfig {
    /// Namespace to watch (empty = all namespaces)
    pub watch_namespace: String,

    pub reconcile_interval_secs: u64,

    pub leader_election: bool,

    pub metrics_bind_address: String,

    pub health_probe_bind_address: String,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            watch_namespace: String::new(),
            reconcile_interval_secs: 30,
            leader_election: true,
            metrics_bind_address: ":8080".to_string(),
            health_probe_bind_address: ":8081".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct MiniKVClusterState {
    name: String,
    namespace: String,
    spec: MiniKVClusterSpec,
    status: MiniKVClusterStatus,
    generation: u64,
    last_reconciled_generation: u64,
}

impl MiniKVController {
    pub fn new(config: ControllerConfig) -> Self {
        Self {
            config,
            clusters: Arc::new(RwLock::new(BTreeMap::new())),
        }
    }

    pub async fn run(&self) -> Result<()> {
        tracing::info!(
            "Starting MiniKV Kubernetes Operator (namespace: {})",
            if self.config.watch_namespace.is_empty() {
                "all"
            } else {
                &self.config.watch_namespace
            }
        );

        tracing::info!("Controller started successfully");
        Ok(())
    }

    pub async fn reconcile(&self, name: &str, namespace: &str) -> Result<ReconcileAction> {
        tracing::info!("Reconciling MiniKVCluster {}/{}", namespace, name);

        let clusters = self.clusters.read().await;
        let key = format!("{}/{}", namespace, name);

        let cluster = match clusters.get(&key) {
            Some(c) => c.clone(),
            None => {
                tracing::warn!("Cluster {} not found", key);
                return Ok(ReconcileAction::Skip);
            }
        };
        drop(clusters);

        if cluster.generation == cluster.last_reconciled_generation {
            return Ok(ReconcileAction::RequeueAfter(
                std::time::Duration::from_secs(self.config.reconcile_interval_secs),
            ));
        }

        self.reconcile_coordinators(&cluster).await?;

        self.reconcile_volumes(&cluster).await?;

        self.reconcile_services(&cluster).await?;

        self.reconcile_config(&cluster).await?;

        if cluster.spec.autoscaling.enabled {
            self.reconcile_autoscaling(&cluster).await?;
        }

        if cluster.spec.backup.enabled {
            self.reconcile_backup(&cluster).await?;
        }

        self.update_status(name, namespace).await?;

        tracing::info!("Reconciliation complete for {}/{}", namespace, name);

        Ok(ReconcileAction::RequeueAfter(
            std::time::Duration::from_secs(self.config.reconcile_interval_secs),
        ))
    }

    async fn reconcile_coordinators(&self, cluster: &MiniKVClusterState) -> Result<()> {
        tracing::debug!(
            "Reconciling coordinators for {}/{}",
            cluster.namespace,
            cluster.name
        );

        let _sts_spec = self.generate_coordinator_statefulset(cluster);

        Ok(())
    }

    fn generate_coordinator_statefulset(&self, cluster: &MiniKVClusterState) -> StatefulSetSpec {
        let spec = &cluster.spec.coordinators;
        let name = format!("{}-coordinator", cluster.name);

        StatefulSetSpec {
            name,
            namespace: cluster.namespace.clone(),
            replicas: spec.replicas,
            image: spec.image.clone(),
            resources: spec.resources.clone(),
            storage: spec.storage.clone(),
            labels: self.generate_labels(&cluster.name, "coordinator"),
            env: self.generate_coordinator_env(cluster),
            ports: vec![
                ContainerPort {
                    name: "http".to_string(),
                    port: 8080,
                },
                ContainerPort {
                    name: "grpc".to_string(),
                    port: 5000,
                },
                ContainerPort {
                    name: "raft".to_string(),
                    port: 5001,
                },
            ],
        }
    }

    fn generate_coordinator_env(&self, cluster: &MiniKVClusterState) -> Vec<EnvVar> {
        let mut env = vec![
            EnvVar {
                name: "MINIKV_NODE_ROLE".to_string(),
                value: "coordinator".to_string(),
            },
            EnvVar {
                name: "MINIKV_CLUSTER_NAME".to_string(),
                value: cluster.name.clone(),
            },
        ];

        if cluster.spec.security.tls.enabled {
            env.push(EnvVar {
                name: "MINIKV_TLS_ENABLED".to_string(),
                value: "true".to_string(),
            });
        }

        if cluster.spec.security.authentication.enabled {
            env.push(EnvVar {
                name: "MINIKV_AUTH_ENABLED".to_string(),
                value: "true".to_string(),
            });
        }

        if cluster.spec.geo.enabled {
            env.push(EnvVar {
                name: "MINIKV_GEO_REGION".to_string(),
                value: cluster.spec.geo.region.clone(),
            });
            env.push(EnvVar {
                name: "MINIKV_GEO_ZONE".to_string(),
                value: cluster.spec.geo.zone.clone(),
            });
        }

        env
    }

    async fn reconcile_volumes(&self, cluster: &MiniKVClusterState) -> Result<()> {
        tracing::debug!(
            "Reconciling volumes for {}/{}",
            cluster.namespace,
            cluster.name
        );

        let _sts_spec = self.generate_volume_statefulset(cluster);

        Ok(())
    }

    fn generate_volume_statefulset(&self, cluster: &MiniKVClusterState) -> StatefulSetSpec {
        let spec = &cluster.spec.volumes;
        let name = format!("{}-volume", cluster.name);

        StatefulSetSpec {
            name,
            namespace: cluster.namespace.clone(),
            replicas: spec.replicas,
            image: spec.image.clone(),
            resources: spec.resources.clone(),
            storage: spec.storage.clone(),
            labels: self.generate_labels(&cluster.name, "volume"),
            env: self.generate_volume_env(cluster),
            ports: vec![
                ContainerPort {
                    name: "http".to_string(),
                    port: 8080,
                },
                ContainerPort {
                    name: "grpc".to_string(),
                    port: 6000,
