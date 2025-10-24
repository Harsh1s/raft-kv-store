//! Time-series storage engine.

use crate::common::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesConfig {
    #[serde(default)]
    pub enabled: bool,

    #[serde(default = "default_retention_days")]
    pub retention_days: u32,

    #[serde(default)]
    pub downsample_rules: Vec<DownsampleRule>,

    #[serde(default)]
    pub compression: CompressionConfig,

    #[serde(default = "default_max_points")]
    pub max_points_per_query: usize,

    #[serde(default = "default_job_interval")]
    pub job_interval_secs: u64,
}

fn default_retention_days() -> u32 {
    30
}

fn default_max_points() -> usize {
    10000
}

fn default_job_interval() -> u64 {
    3600
}

impl Default for TimeseriesConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            retention_days: 30,
            downsample_rules: vec![
                DownsampleRule {
                    after: Duration::days(7),
                    resolution: Resolution::Hour,
                    aggregation: Aggregation::Average,
                },
                DownsampleRule {
                    after: Duration::days(30),
                    resolution: Resolution::Day,
                    aggregation: Aggregation::Average,
                },
            ],
            compression: CompressionConfig::default(),
            max_points_per_query: 10000,
            job_interval_secs: 3600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownsampleRule {
    #[serde(with = "duration_serde")]
    pub after: Duration,

    pub resolution: Resolution,

    pub aggregation: Aggregation,
}

mod duration_serde {
    use chrono::Duration;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(duration.num_seconds())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = i64::deserialize(deserializer)?;
        Ok(Duration::seconds(secs))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
}

impl Resolution {
    pub fn as_millis(&self) -> i64 {
        match self {
            Resolution::Second => 1_000,
            Resolution::Minute => 60_000,
            Resolution::Hour => 3_600_000,
            Resolution::Day => 86_400_000,
            Resolution::Week => 604_800_000,
            Resolution::Month => 2_592_000_000, // ~30 days
        }
    }

    pub fn align(&self, ts: i64) -> i64 {
        let millis = self.as_millis();
        (ts / millis) * millis
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Aggregation {
    Average,
    Sum,
    Min,
    Max,
    Count,
    First,
    Last,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    #[serde(default = "default_true")]
    pub delta_encoding: bool,

    #[serde(default = "default_true")]
    pub run_length_encoding: bool,

    #[serde(default = "default_true")]
    pub gorilla_compression: bool,
}

fn default_true() -> bool {
    true
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            delta_encoding: true,
            run_length_encoding: true,
            gorilla_compression: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: i64,
    pub value: f64,
}

impl DataPoint {
    pub fn new(timestamp: i64, value: f64) -> Self {
        Self { timestamp, value }
    }

    pub fn now(value: f64) -> Self {
        Self {
            timestamp: Utc::now().timestamp_millis(),
            value,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeries {
    pub metric: String,

    #[serde(default)]
    pub tags: HashMap<String, String>,

    pub points: Vec<DataPoint>,
}

impl TimeSeries {
    pub fn new(metric: &str) -> Self {
        Self {
            metric: metric.to_string(),
            tags: HashMap::new(),
            points: Vec::new(),
        }
    }

    pub fn with_tag(mut self, key: &str, value: &str) -> Self {
        self.tags.insert(key.to_string(), value.to_string());
        self
    }

    pub fn add_point(&mut self, point: DataPoint) {
        self.points.push(point);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesQuery {
    pub metric: String,

    pub start: DateTime<Utc>,

    pub end: DateTime<Utc>,

    #[serde(default)]
    pub tags: HashMap<String, String>,

    #[serde(default)]
    pub aggregation: Option<Aggregation>,

    #[serde(default)]
    pub resolution: Option<Resolution>,

    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeseriesResult {
    pub series: Vec<TimeSeries>,

    pub execution_time_ms: u64,

    pub points_scanned: u64,

    pub points_returned: u64,
}

pub struct TimeseriesEngine {
    config: TimeseriesConfig,
    data: Arc<RwLock<BTreeMap<String, BTreeMap<i64, CompressedBlock>>>>,
    /// Index: metric+tags -> series ID
    index: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

#[derive(Debug, Clone)]
pub struct CompressedBlock {
