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
