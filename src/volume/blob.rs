//! BlobStore implementation
//! This module provides the main storage engine for data volumes.
//! It uses a log-structured, append-only design for durability and performance.
//! The in-memory HashMap index enables fast lookups, while a Bloom filter accelerates negative lookups.
//! All operations are logged to a Write-Ahead Log (WAL) for crash recovery.
//!
//! Features:
//! - TTL (Time-To-Live) support for automatic key expiration
//! - LZ4 compression for efficient storage
//! - Background cleanup task for expired keys

use crate::common::{blake3_hash, crc32, Result, WalSyncPolicy};
use crate::volume::index::{BlobLocation, Index};
use crate::volume::wal::{Wal, WalEntry, WalOp};
use bloomfilter::Bloom;
use std::fs::{self, File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

const BLOB_MAGIC: [u8; 4] = [0x42, 0x4C, 0x4F, 0x42];
const BLOB_MAGIC_COMPRESSED: [u8; 4] = [0x42, 0x4C, 0x4F, 0x43]; // BLOC
const SEGMENT_SIZE: u64 = 64 * 1024 * 1024;
const MAX_SEGMENTS: u64 = 1000;
const COMPRESSION_THRESHOLD: usize = 128;

#[derive(Debug, Clone)]
pub struct StoreStats {
    pub total_keys: usize,
    pub total_bytes: u64,
    pub active_segments: usize,
    pub index_size: usize,
    pub bloom_false_positives: u64,
    pub keys_with_ttl: usize,
    pub compressed_blobs: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CompressionMode {
    #[default]
    None,
    Lz4,
}

pub struct BlobStore {
    data_path: PathBuf,

    index: Index,
    bloom: Bloom<[u8; 32]>,
    wal: Wal,
    current_segment: u64,
    current_offset: u64,
    sync_policy: WalSyncPolicy,
    compression: CompressionMode,
}

impl BlobStore {
    pub fn open(data_path: &Path, wal_path: &Path, sync_policy: WalSyncPolicy) -> Result<Self> {
        fs::create_dir_all(data_path)?;
        fs::create_dir_all(wal_path)?;

        let snapshot_path = data_path.join("index.snap");
        let mut index = if snapshot_path.exists() {
            Index::load_snapshot(&snapshot_path)?
        } else {
            Index::new()
        };

        let bloom_path = data_path.join("bloom.filter");
        let mut bloom = if bloom_path.exists() {
            let bytes = fs::read(&bloom_path)?;
            Bloom::from_bytes(bytes)
                .unwrap_or_else(|_: &str| Bloom::new_for_fp_rate(100_000, 0.01).unwrap())
        } else {
            Bloom::new_for_fp_rate(100_000, 0.01).unwrap()
        };

        let wal_file = wal_path.join("wal.log");
        let wal = Wal::open(&wal_file, sync_policy)?;

        Wal::replay(&wal_file, &mut |entry: WalEntry| {
            match entry.op {
                WalOp::Put { ref key, .. } => {
                    let hash = blake3_hash(key.as_bytes());
                    let hash_vec: Vec<u8> = hex::decode(&hash).unwrap_or_else(|_| vec![0u8; 32]);
                    let hash_bytes: [u8; 32] = hash_vec.try_into().unwrap_or([0u8; 32]);
                    bloom.set(&hash_bytes);
                }
                WalOp::Delete { ref key } => {
                    index.remove(key);
                }
            }
            Ok(())
        })?;

        if !snapshot_path.exists() {
            Self::rebuild_index_from_segments(&mut index, &mut bloom, data_path)?;
        } else {
            for key in index.keys() {
                let hash = blake3_hash(key.as_bytes());
                let hash_vec: Vec<u8> = hex::decode(&hash).unwrap_or_else(|_| vec![0u8; 32]);
                let hash_bytes: [u8; 32] = hash_vec.try_into().unwrap_or([0u8; 32]);
                bloom.set(&hash_bytes);
            }
        }

        let (current_segment, current_offset) = Self::find_current_position(data_path)?;

        Ok(Self {
            data_path: data_path.to_path_buf(),

            index,
            bloom,
            wal,
            current_segment,
            current_offset,
            sync_policy,
            compression: CompressionMode::None,
        })
    }

    pub fn open_with_compression(
        data_path: &Path,
        wal_path: &Path,
        sync_policy: WalSyncPolicy,
        compression: CompressionMode,
    ) -> Result<Self> {
        let mut store = Self::open(data_path, wal_path, sync_policy)?;
        store.compression = compression;
        Ok(store)
    }

    pub fn set_compression(&mut self, mode: CompressionMode) {
        self.compression = mode;
    }

    pub fn compression_mode(&self) -> CompressionMode {
        self.compression
    }

    pub fn put_with_ttl(&mut self, key: &str, value: &[u8], ttl_ms: Option<u64>) -> Result<()> {
        self.wal.append_put(key, value)?;
        let hash = blake3_hash(key.as_bytes());
        let hash_vec: Vec<u8> = hex::decode(&hash).unwrap_or_else(|_| vec![0u8; 32]);
        let hash_bytes: [u8; 32] = hash_vec.try_into().unwrap_or([0u8; 32]);
        self.bloom.set(&hash_bytes);

        let mut location = self.write_blob(key, value)?;

        if let Some(ttl) = ttl_ms {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            location.expires_at = Some(now + ttl);
        }

        self.index.insert(key.to_string(), location);
        Ok(())
    }

    pub fn put(&mut self, key: &str, value: &[u8]) -> Result<()> {
        self.put_with_ttl(key, value, None)
    }

    pub fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let hash = blake3_hash(key.as_bytes());
        let hash_vec: Vec<u8> = hex::decode(&hash).unwrap_or_else(|_| vec![0u8; 32]);
        let hash_bytes: [u8; 32] = hash_vec.try_into().unwrap_or([0u8; 32]);

        if !self.bloom.check(&hash_bytes) {
            return Ok(None);
        }

        match self.index.get_if_valid(key) {
            Some(loc) => self.read_blob(loc),
            None => Ok(None),
        }
    }

    pub fn delete(&mut self, key: &str) -> Result<()> {
        self.wal.append_delete(key)?;
        self.index.remove(key);
        Ok(())
    }

    pub fn compact(&mut self) -> Result<()> {
        let temp_path = self.data_path.join("compact_temp");
        fs::create_dir_all(&temp_path)?;

        let mut new_index = Index::new();
        let mut new_segment = 0u64;
