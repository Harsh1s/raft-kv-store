//! In-memory index for fast key lookups
//!
//! This module provides a fast, in-memory HashMap index for key-value lookups.
//! Each key maps to a BlobLocation, which describes where the value is stored on disk.
//! The index supports snapshotting for fast recovery after a crash.
//! TTL (Time-To-Live) support enables automatic key expiration.

use crate::common::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

const SNAPSHOT_MAGIC: &[u8; 8] = b"KVINDEX3"; // Bumped version for TTL support

/// Describes the physical location of a value in the log-structured storage engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlobLocation {
    pub shard: u64,
    pub offset: u64,
    pub size: u64,
    pub blake3: String,
    #[serde(default)]
    pub expires_at: Option<u64>,
}

#[derive(Debug, Default)]
pub struct Index {
    map: HashMap<String, BlobLocation>,
}

impl Index {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, location: BlobLocation) {
        self.map.insert(key, location);
    }

    pub fn get(&self, key: &str) -> Option<&BlobLocation> {
        self.map.get(key)
    }

    pub fn remove(&mut self, key: &str) -> Option<BlobLocation> {
        self.map.remove(key)
    }

    pub fn contains(&self, key: &str) -> bool {
        self.map.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.map.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &BlobLocation)> {
        self.map.iter()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    pub fn is_expired(&self, key: &str) -> bool {
        if let Some(loc) = self.map.get(key) {
            if let Some(expires_at) = loc.expires_at {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                return now > expires_at;
            }
        }
        false
    }

    pub fn get_if_valid(&self, key: &str) -> Option<&BlobLocation> {
        if self.is_expired(key) {
            return None;
        }
        self.map.get(key)
    }

    pub fn cleanup_expired(&mut self) -> usize {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let expired_keys: Vec<String> = self
            .map
            .iter()
            .filter(|(_, loc)| loc.expires_at.map(|exp| now > exp).unwrap_or(false))
            .map(|(k, _)| k.clone())
            .collect();

        let count = expired_keys.len();
        for key in expired_keys {
            self.map.remove(&key);
        }
        count
    }

    pub fn keys_with_ttl(&self) -> Vec<(&String, u64)> {
        self.map
            .iter()
            .filter_map(|(k, loc)| loc.expires_at.map(|exp| (k, exp)))
            .collect()
    }

    pub fn save_snapshot(&self, path: impl AsRef<Path>) -> Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writer.write_all(SNAPSHOT_MAGIC)?;

        writer.write_all(&(self.map.len() as u64).to_le_bytes())?;

        for (key, loc) in &self.map {
            let key_bytes = key.as_bytes();
            writer.write_all(&(key_bytes.len() as u32).to_le_bytes())?;
