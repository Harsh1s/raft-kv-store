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
