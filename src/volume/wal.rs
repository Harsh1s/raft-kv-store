//! Write-Ahead Log (WAL) implementation
//!
//! Ensures durability by writing operations to a log before applying them.
//! WAL format: [MAGIC][SEQUENCE][OP][KEY_LEN][VALUE_LEN][KEY][VALUE][CRC32]
//!
//! This module provides append-only logging for all write and delete operations.
//! On recovery, the log is replayed to restore the latest state.

use crate::common::{crc32, Error, Result, WalSyncPolicy};
use std::fs::{File, OpenOptions};
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

const WAL_MAGIC: [u8; 4] = [0x57, 0x41, 0x4C, 0x31]; // "WAL1"
const OP_PUT: u8 = 1;
const OP_DELETE: u8 = 2;

/// Represents a single operation in the log, either a write (Put) or a delete.
#[derive(Debug, Clone)]
pub struct WalEntry {
    pub sequence: u64,
    pub op: WalOp,
}

#[derive(Debug, Clone)]
pub enum WalOp {
    Put { key: String, value: Vec<u8> },
    Delete { key: String },
}

/// Main WAL structure. Handles appending operations and syncing to disk.
pub struct Wal {
    path: PathBuf,
    writer: BufWriter<File>,
    next_sequence: u64,
    sync_policy: WalSyncPolicy,
}

impl Wal {
    /// If the file exists, finds the last sequence number to continue appending.
    pub fn open(path: impl AsRef<Path>, sync_policy: WalSyncPolicy) -> Result<Self> {
        let path = path.as_ref().to_path_buf();

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&path)?;

        let next_sequence = Self::find_last_sequence(&path)?;

        Ok(Self {
            path,
            writer: BufWriter::new(file),
            next_sequence,
            sync_policy,
        })
    }

    fn find_last_sequence(path: &Path) -> Result<u64> {
        let file = match File::open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e.into()),
        };

        let mut reader = BufReader::new(file);
        let mut max_seq = None;

        loop {
            match Self::read_entry_internal(&mut reader) {
                Ok(Some(entry)) => {
                    max_seq = Some(max_seq.unwrap_or(0).max(entry.sequence));
                }
                Ok(None) => break,
                Err(_) => break, // Corrupted entry, stop reading
            }
        }

        Ok(max_seq.map(|s| s + 1).unwrap_or(0))
    }

    pub fn append_put(&mut self, key: &str, value: &[u8]) -> Result<u64> {
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        self.write_entry(sequence, OP_PUT, key, Some(value))?;
        self.maybe_sync()?;

        Ok(sequence)
    }

    pub fn append_delete(&mut self, key: &str) -> Result<u64> {
        let sequence = self.next_sequence;
        self.next_sequence += 1;

        self.write_entry(sequence, OP_DELETE, key, None)?;
        self.maybe_sync()?;

        Ok(sequence)
    }

    fn write_entry(
        &mut self,
        sequence: u64,
        op: u8,
        key: &str,
        value: Option<&[u8]>,
    ) -> Result<()> {
        let key_bytes = key.as_bytes();
        let val_bytes = value.unwrap_or(&[]);

        self.writer.write_all(&WAL_MAGIC)?;
        self.writer.write_all(&sequence.to_le_bytes())?;
        self.writer.write_all(&[op])?;
