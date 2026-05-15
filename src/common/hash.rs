//! Hashing utilities (BLAKE3, HRW, sharding).

use blake3::Hasher;
use std::collections::HashMap;

pub fn blake3_hash(data: &[u8]) -> String {
    let hash = blake3::hash(data);
    format!("{}", hash)
}

pub struct Blake3Hasher {
    hasher: Hasher,
}

impl Blake3Hasher {
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    pub fn finalize(&self) -> String {
        let hash = self.hasher.finalize();
        format!("{}", hash)
    }
}

impl Default for Blake3Hasher {
    fn default() -> Self {
        Self::new()
    }
}

pub fn shard_key(key: &str, num_shards: u64) -> u64 {
    let hash = blake3::hash(key.as_bytes());
    let hash_u64 = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());
    hash_u64 % num_shards
}

///
/// (deterministic based on key).  This ensures consistent placement even
/// as the cluster changes.
pub fn hrw_hash(key: &str, nodes: &[String]) -> Vec<String> {
    let mut weights: Vec<(String, u64)> = nodes
        .iter()
        .map(|node| {
            let combined = format!("{}{}", key, node);
            let hash = blake3::hash(combined.as_bytes());
            let weight = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());
            (node.clone(), weight)
        })
        .collect();

    weights.sort_by_key(|b| std::cmp::Reverse(b.1));

    weights.into_iter().map(|(node, _)| node).collect()
}

pub fn select_replicas(key: &str, nodes: &[String], n: usize) -> Vec<String> {
    let sorted = hrw_hash(key, nodes);
    sorted.into_iter().take(n).collect()
}

pub fn blob_prefix(key: &str) -> (String, String) {
    let hash = blake3::hash(key.as_bytes());
    let bytes = hash.as_bytes();
    (format!("{:02x}", bytes[0]), format!("{:02x}", bytes[1]))
}

/// Maps keys to shards, and shards to nodes. Supports rebalancing
/// when nodes are added/removed.
pub struct ConsistentHashRing {
    pub num_shards: u64,
