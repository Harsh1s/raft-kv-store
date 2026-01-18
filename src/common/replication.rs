//! Cross-datacenter replication with async replication and conflict resolution.

use crate::common::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

pub type DatacenterId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// This datacenter's ID
    pub local_dc: DatacenterId,

    pub remote_dcs: Vec<RemoteDatacenter>,

    #[serde(default)]
    pub conflict_resolution: ConflictResolution,

    #[serde(default = "default_true")]
    pub async_replication: bool,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    #[serde(default = "default_replication_interval")]
    pub replication_interval_ms: u64,

    #[serde(default = "default_max_lag")]
    pub max_lag_secs: u64,
}

fn default_true() -> bool {
    true
}

fn default_batch_size() -> usize {
    100
}

fn default_replication_interval() -> u64 {
    1000
}

fn default_max_lag() -> u64 {
    60
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            local_dc: "dc1".to_string(),
            remote_dcs: vec![],
            conflict_resolution: ConflictResolution::default(),
            async_replication: true,
            batch_size: 100,
            replication_interval_ms: 1000,
            max_lag_secs: 60,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteDatacenter {
    pub id: DatacenterId,

    pub name: String,

    pub endpoints: Vec<String>,

    #[serde(default)]
    pub priority: u32,

    #[serde(default)]
    pub region: String,

    #[serde(default)]
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolution {
    #[default]
    LastWriteWins,

    VectorClock,

    LocalFirst,

    PrimaryFirst,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VectorClock {
    pub clocks: HashMap<DatacenterId, u64>,
}

impl VectorClock {
    pub fn new() -> Self {
        Self {
            clocks: HashMap::new(),
        }
    }

    pub fn increment(&mut self, dc: &DatacenterId) {
        *self.clocks.entry(dc.clone()).or_insert(0) += 1;
    }

    pub fn merge(&mut self, other: &VectorClock) {
        for (dc, &ts) in &other.clocks {
            let entry = self.clocks.entry(dc.clone()).or_insert(0);
            *entry = (*entry).max(ts);
        }
    }

    pub fn is_concurrent(&self, other: &VectorClock) -> bool {
        let self_dominates = self.dominates(other);
        let other_dominates = other.dominates(self);
        !self_dominates && !other_dominates
    }

    pub fn dominates(&self, other: &VectorClock) -> bool {
        let mut dominated = false;
        for (dc, &ts) in &other.clocks {
            let self_ts = self.clocks.get(dc).copied().unwrap_or(0);
            if self_ts < ts {
                return false;
            }
            if self_ts > ts {
                dominated = true;
            }
        }
        dominated
    }

    pub fn get(&self, dc: &DatacenterId) -> u64 {
        self.clocks.get(dc).copied().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationEvent {
    pub id: String,

    pub source_dc: DatacenterId,

    pub event_type: ReplicationEventType,

    pub key: String,

    pub value: Option<Vec<u8>>,

    pub timestamp: DateTime<Utc>,

    pub vector_clock: Option<VectorClock>,

    pub tenant: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationEventType {
    Put,
    Delete,
    Snapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStatus {
    pub dc_id: DatacenterId,

    pub last_replicated_id: Option<String>,

    pub last_replicated_at: Option<DateTime<Utc>>,

    pub lag_secs: u64,

    pub pending_events: usize,

    pub healthy: bool,

    pub last_error: Option<String>,
}

pub struct ReplicationManager {
    config: ReplicationConfig,

    pending_events: Arc<RwLock<Vec<ReplicationEvent>>>,

    status: Arc<RwLock<HashMap<DatacenterId, ReplicationStatus>>>,

    event_tx: mpsc::Sender<ReplicationEvent>,

    shutdown: Arc<RwLock<bool>>,
}

impl ReplicationManager {
    pub fn new(config: ReplicationConfig) -> (Self, mpsc::Receiver<ReplicationEvent>) {
        let (event_tx, event_rx) = mpsc::channel(10000);

        let mut status = HashMap::new();
        for dc in &config.remote_dcs {
            status.insert(
                dc.id.clone(),
                ReplicationStatus {
                    dc_id: dc.id.clone(),
                    last_replicated_id: None,
                    last_replicated_at: None,
                    lag_secs: 0,
                    pending_events: 0,
                    healthy: true,
                    last_error: None,
                },
            );
        }

        let manager = Self {
            config,
            pending_events: Arc::new(RwLock::new(Vec::new())),
            status: Arc::new(RwLock::new(status)),
            event_tx,
            shutdown: Arc::new(RwLock::new(false)),
        };

        (manager, event_rx)
    }

    pub async fn queue_event(&self, event: ReplicationEvent) -> Result<()> {
        self.event_tx
            .send(event.clone())
            .await
            .map_err(|e| Error::Other(format!("Failed to queue replication event: {}", e)))?;

        let mut pending = self.pending_events.write().unwrap();
        pending.push(event);

        Ok(())
    }

    pub fn create_put_event(
        &self,
        key: &str,
        value: Vec<u8>,
        tenant: Option<String>,
    ) -> ReplicationEvent {
        ReplicationEvent {
            id: uuid::Uuid::new_v4().to_string(),
            source_dc: self.config.local_dc.clone(),
            event_type: ReplicationEventType::Put,
            key: key.to_string(),
            value: Some(value),
            timestamp: Utc::now(),
            vector_clock: None,
            tenant,
        }
    }

    pub fn create_delete_event(&self, key: &str, tenant: Option<String>) -> ReplicationEvent {
        ReplicationEvent {
            id: uuid::Uuid::new_v4().to_string(),
            source_dc: self.config.local_dc.clone(),
            event_type: ReplicationEventType::Delete,
            key: key.to_string(),
            value: None,
            timestamp: Utc::now(),
            vector_clock: None,
            tenant,
        }
    }

    pub fn get_status(&self) -> Vec<ReplicationStatus> {
        self.status.read().unwrap().values().cloned().collect()
    }

    pub fn get_dc_status(&self, dc_id: &str) -> Option<ReplicationStatus> {
        self.status.read().unwrap().get(dc_id).cloned()
    }

    pub fn update_status(&self, dc_id: &str, update: impl FnOnce(&mut ReplicationStatus)) {
        if let Some(status) = self.status.write().unwrap().get_mut(dc_id) {
            update(status);
        }
    }

    pub fn resolve_conflict<'a>(
