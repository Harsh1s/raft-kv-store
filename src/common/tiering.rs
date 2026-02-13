//! Data tiering (hot/warm/cold/archive).

use crate::common::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieringConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default)]
    pub hot: TierConfig,

    #[serde(default)]
    pub warm: TierConfig,

    #[serde(default)]
    pub cold: TierConfig,

    pub archive: Option<ArchiveConfig>,

    #[serde(default)]
    pub policies: Vec<TieringPolicy>,

    #[serde(default = "default_interval")]
    pub interval_secs: u64,
}

fn default_interval() -> u64 {
    3600 // 1 hour
}

impl Default for TieringConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            hot: TierConfig::default_hot(),
            warm: TierConfig::default_warm(),
            cold: TierConfig::default_cold(),
            archive: None,
            policies: vec![],
            interval_secs: default_interval(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TierConfig {
    pub name: String,

    pub path: Option<PathBuf>,

    /// Maximum size in bytes (0 = unlimited)
    #[serde(default)]
    pub max_size_bytes: u64,

    /// Maximum number of items (0 = unlimited)
    #[serde(default)]
    pub max_items: u64,

    #[serde(default)]
    pub compression: bool,

    #[serde(default)]
    pub compression_algorithm: CompressionAlgorithm,

    #[serde(default)]
    pub cache_enabled: bool,

    #[serde(default)]
    pub cache_size_bytes: u64,
}

impl TierConfig {
    fn default_hot() -> Self {
        Self {
            name: "hot".to_string(),
            path: None,
            max_size_bytes: 1024 * 1024 * 1024, // 1 GB
            max_items: 0,
            compression: false,
            compression_algorithm: CompressionAlgorithm::None,
            cache_enabled: true,
            cache_size_bytes: 256 * 1024 * 1024, // 256 MB
        }
    }

    fn default_warm() -> Self {
        Self {
            name: "warm".to_string(),
            path: None,
            max_size_bytes: 10 * 1024 * 1024 * 1024, // 10 GB
            max_items: 0,
            compression: true,
            compression_algorithm: CompressionAlgorithm::Lz4,
            cache_enabled: false,
            cache_size_bytes: 0,
        }
    }

    fn default_cold() -> Self {
        Self {
            name: "cold".to_string(),
            path: None,
            max_size_bytes: 100 * 1024 * 1024 * 1024, // 100 GB
            max_items: 0,
            compression: true,
            compression_algorithm: CompressionAlgorithm::Zstd,
            cache_enabled: false,
            cache_size_bytes: 0,
        }
    }
}

impl Default for TierConfig {
    fn default() -> Self {
        Self::default_hot()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    pub endpoint: String,

    pub bucket: String,

    pub access_key: Option<String>,

    pub secret_key: Option<String>,

    pub region: Option<String>,

    #[serde(default = "default_true")]
    pub compression: bool,

    #[serde(default = "default_storage_class")]
    pub storage_class: String,
}

fn default_true() -> bool {
    true
}

fn default_storage_class() -> String {
    "STANDARD".to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CompressionAlgorithm {
    #[default]
    None,
    Lz4,
    Zstd,
    Snappy,
    Gzip,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieringPolicy {
    pub name: String,

    pub prefix: Option<String>,

    pub rules: Vec<TieringRule>,

    /// Priority (higher = evaluated first)
    #[serde(default)]
    pub priority: i32,

    /// Is policy enabled?
    #[serde(default = "default_true")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TieringRule {
    pub condition: TieringCondition,

    pub target_tier: Tier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tier {
    Hot,
    Warm,
    Cold,
    Archive,
}

impl Tier {
    pub fn temperature(&self) -> i32 {
        match self {
            Tier::Hot => 3,
            Tier::Warm => 2,
            Tier::Cold => 1,
            Tier::Archive => 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TieringCondition {
    AccessCount {
        min_count: Option<u64>,
        max_count: Option<u64>,
        window_secs: u64,
    },

    Age {
        min_age_secs: Option<u64>,
        max_age_secs: Option<u64>,
    },

    Size {
        min_bytes: Option<u64>,
        max_bytes: Option<u64>,
    },

    LastAccess {
        min_since_access_secs: Option<u64>,
        max_since_access_secs: Option<u64>,
    },

    And {
        conditions: Vec<TieringCondition>,
    },

    Or {
        conditions: Vec<TieringCondition>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMetadata {
    pub key: String,

    pub tier: Tier,

    pub size_bytes: u64,

    pub created_at: DateTime<Utc>,
