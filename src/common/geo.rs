//! Geo-partitioning and region-aware routing.

use crate::common::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoConfig {
    #[serde(default)]
    pub enabled: bool,

    /// This node's region
    pub local_region: String,

    /// This node's zone (availability zone)
    #[serde(default)]
    pub local_zone: String,

    #[serde(default)]
    pub regions: Vec<RegionConfig>,

    #[serde(default)]
    pub default_region: Option<String>,

    #[serde(default)]
    pub routing: RoutingStrategy,

    #[serde(default)]
    pub geo_fencing: bool,

    #[serde(default)]
    pub replication_mode: ReplicationMode,
}

impl Default for GeoConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            local_region: "default".to_string(),
            local_zone: "".to_string(),
            regions: vec![],
            default_region: None,
            routing: RoutingStrategy::default(),
            geo_fencing: false,
            replication_mode: ReplicationMode::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionConfig {
    /// Region identifier (e.g., "us-east-1", "eu-west-1")
    pub id: String,

    pub name: String,

    #[serde(default)]
    pub location: Option<GeoLocation>,

    pub endpoints: Vec<String>,

    /// Is this the primary region?
    #[serde(default)]
    pub primary: bool,

    /// Priority for failover (lower = higher priority)
    #[serde(default)]
    pub priority: u32,

    #[serde(default)]
    pub settings: RegionSettings,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GeoLocation {
    pub latitude: f64,
    pub longitude: f64,
}

impl GeoLocation {
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }

    pub fn distance_km(&self, other: &GeoLocation) -> f64 {
        const EARTH_RADIUS_KM: f64 = 6371.0;

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegionSettings {
    #[serde(default)]
    pub read_replicas: u32,

    #[serde(default = "default_true")]
    pub write_enabled: bool,

    #[serde(default)]
    pub compliance: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    #[default]
    LatencyBased,

    KeyBased,

    /// Route based on client's geographic location
    GeoBased,

    RoundRobin,

    PrimaryOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ReplicationMode {
    Sync,

    #[default]
    Async,

    SemiSync,
}

pub struct GeoRouter {
    config: GeoConfig,
    region_health: Arc<RwLock<HashMap<String, RegionHealth>>>,
    latencies: Arc<RwLock<HashMap<String, LatencyStats>>>,
    ip_location_cache: Arc<RwLock<HashMap<IpAddr, GeoLocation>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionHealth {
    pub region_id: String,
    pub healthy: bool,
    pub available_endpoints: usize,
    pub last_check: DateTime<Utc>,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct LatencyStats {
    pub samples: Vec<f64>,
    pub avg_ms: f64,
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub p99_ms: f64,
}

impl LatencyStats {
    pub fn add_sample(&mut self, latency_ms: f64) {
        self.samples.push(latency_ms);
        if self.samples.len() > 1000 {
            self.samples.remove(0);
        }
        self.recalculate();
    }

    fn recalculate(&mut self) {
        if self.samples.is_empty() {
            return;
        }

        let mut sorted = self.samples.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        self.avg_ms = sorted.iter().sum::<f64>() / sorted.len() as f64;
        self.p50_ms = sorted[sorted.len() / 2];
        self.p95_ms = sorted[(sorted.len() as f64 * 0.95) as usize];
        self.p99_ms = sorted[(sorted.len() as f64 * 0.99) as usize];
    }
}

impl GeoRouter {
    pub fn new(config: GeoConfig) -> Self {
        Self {
            config,
            region_health: Arc::new(RwLock::new(HashMap::new())),
            latencies: Arc::new(RwLock::new(HashMap::new())),
            ip_location_cache: Arc::new(RwLock::new(HashMap::new())),
        }
