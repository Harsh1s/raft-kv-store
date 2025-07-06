//! Placement strategy using HRW hashing and sharding
//!
//! This module implements horizontal scaling via sharding and flexible replica sets.
//! Keys are assigned to shards using HRW (Highest Random Weight) hashing, and replicas are selected for fault tolerance.

use crate::common::{select_replicas, shard_key, ConsistentHashRing, Result};
use crate::coordinator::metadata::VolumeMetadata;

/// PlacementManager handles sharding and replica selection for distributed writes.
pub struct PlacementManager {
    ring: ConsistentHashRing,
    replicas: usize,
    num_shards: u64,
}

impl PlacementManager {
    pub fn new(num_shards: u64, replicas: usize) -> Self {
        Self {
            ring: ConsistentHashRing::new(num_shards),
            replicas,
            num_shards,
        }
    }

    /// Uses HRW hashing to assign the key to a shard and select healthy replicas.
    pub fn select_volumes(&self, key: &str, volumes: &[VolumeMetadata]) -> Result<Vec<String>> {
        if volumes.is_empty() {
            return Err(crate::Error::NoHealthyVolumes);
        }

        let healthy: Vec<String> = volumes
            .iter()
            .filter(|v| v.state.is_healthy())
            .map(|v| v.volume_id.clone())
            .collect();

        if healthy.is_empty() {
            return Err(crate::Error::NoHealthyVolumes);
        }

        let selected = select_replicas(key, &healthy, self.replicas);

        if selected.len() < self.replicas {
            return Err(crate::Error::InsufficientReplicas {
                needed: self.replicas,
                available: selected.len(),
            });
        }

        Ok(selected)
    }

    pub fn get_shard(&self, key: &str) -> u64 {
        shard_key(key, self.num_shards)
    }

    pub fn rebalance(&mut self, volumes: &[VolumeMetadata]) {
        let available: Vec<String> = volumes
            .iter()
            .filter(|v| v.state.is_healthy())
            .map(|v| v.volume_id.clone())
            .collect();

        self.ring.rebalance(&available, self.replicas);
    }

    pub fn get_shard_volumes(&self, shard: u64) -> Option<Vec<String>> {
        self.ring.get_shard_nodes(shard).map(|nodes| nodes.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::NodeState;

    fn mock_volume(id: &str, state: NodeState) -> VolumeMetadata {
        VolumeMetadata {
            volume_id: id.to_string(),
            address: format!("http://localhost:{}", id),
            grpc_address: format!("http://localhost:{}", id),
            state,
            shards: vec![],
            total_keys: 0,
            total_bytes: 0,
            free_bytes: 0,
