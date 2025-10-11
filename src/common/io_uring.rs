//! io_uring I/O backend (Linux 5.1+).

use crate::common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// io_uring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoUringConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_sq_depth")]
    pub sq_depth: u32,

    #[serde(default = "default_cq_depth")]
    pub cq_depth: u32,

    #[serde(default)]
    pub kernel_poll: bool,

    #[serde(default = "default_poll_idle")]
    pub poll_idle_ms: u32,

    #[serde(default = "default_true")]
    pub registered_buffers: bool,

    #[serde(default = "default_buffer_count")]
    pub buffer_count: usize,

    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,

    #[serde(default)]
    pub direct_io: bool,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_sq_depth() -> u32 {
    256
}

fn default_cq_depth() -> u32 {
    512
}

fn default_poll_idle() -> u32 {
    1000
}

fn default_buffer_count() -> usize {
    64
}

fn default_buffer_size() -> usize {
    64 * 1024 // 64 KB
}

fn default_batch_size() -> usize {
    32
}

fn default_true() -> bool {
    true
}

impl Default for IoUringConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            sq_depth: default_sq_depth(),
            cq_depth: default_cq_depth(),
            kernel_poll: false,
            poll_idle_ms: default_poll_idle(),
            registered_buffers: true,
            buffer_count: default_buffer_count(),
            buffer_size: default_buffer_size(),
            direct_io: false,
            batch_size: default_batch_size(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoOpType {
    Read,
    Write,
    Fsync,
    Fdatasync,
}

#[derive(Debug)]
pub struct IoRequest {
    pub op: IoOpType,

    pub fd: i32,

    pub offset: u64,

    pub buffer: Vec<u8>,

    pub user_data: u64,
}

#[derive(Debug)]
pub struct IoResult {
    pub user_data: u64,

    pub result: i32,

    pub op: IoOpType,
}

/// io_uring interface (abstraction for platform compatibility)
pub struct IoUring {
    config: IoUringConfig,

    pending: VecDeque<IoRequest>,

    stats: Arc<IoUringStats>,

    available: bool,
}

/// io_uring statistics
#[derive(Debug, Default)]
pub struct IoUringStats {
    pub submissions: AtomicU64,

    pub completions: AtomicU64,

    pub bytes_read: AtomicU64,

    pub bytes_written: AtomicU64,

    pub batched_submissions: AtomicU64,

    pub avg_batch_size: AtomicU64,

    pub poll_wakeups: AtomicU64,
}

impl IoUring {
    pub fn new(config: IoUringConfig) -> Result<Self> {
        let available = Self::check_availability();

        if config.enabled && !available {
            tracing::warn!("io_uring requested but not available, falling back to standard I/O");
        }

        Ok(Self {
            config,
            pending: VecDeque::new(),
            stats: Arc::new(IoUringStats::default()),
            available,
        })
    }

    fn check_availability() -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            if let Ok(version) = fs::read_to_string("/proc/version") {
                if let Some(ver) = version.split_whitespace().nth(2) {
