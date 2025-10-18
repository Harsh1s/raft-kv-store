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
    }

    pub fn route(&self, request: &GeoRequest) -> Result<RoutingDecision> {
        if !self.config.enabled {
            return Ok(RoutingDecision {
                region: self.config.local_region.clone(),
                endpoints: vec![],
                reason: "Geo-partitioning disabled".to_string(),
            });
        }

        match self.config.routing {
            RoutingStrategy::LatencyBased => self.route_by_latency(request),
            RoutingStrategy::KeyBased => self.route_by_key(request),
            RoutingStrategy::GeoBased => self.route_by_geo(request),
            RoutingStrategy::RoundRobin => self.route_round_robin(request),
            RoutingStrategy::PrimaryOnly => self.route_to_primary(request),
        }
    }

    fn route_by_latency(&self, _request: &GeoRequest) -> Result<RoutingDecision> {
        let latencies = self.latencies.read().unwrap();
        let health = self.region_health.read().unwrap();

        let mut best_region = None;
        let mut best_latency = f64::MAX;

        for region in &self.config.regions {
            if let Some(h) = health.get(&region.id) {
                if !h.healthy {
                    continue;
                }
            }

            if let Some(stats) = latencies.get(&region.id) {
                if stats.avg_ms < best_latency {
                    best_latency = stats.avg_ms;
                    best_region = Some(region.clone());
                }
            }
        }

        match best_region {
            Some(region) => Ok(RoutingDecision {
                region: region.id.clone(),
                endpoints: region.endpoints.clone(),
                reason: format!("Lowest latency: {:.2}ms", best_latency),
            }),
            None => Ok(RoutingDecision {
                region: self.config.local_region.clone(),
                endpoints: vec![],
                reason: "Fallback to local region".to_string(),
            }),
        }
    }

    fn route_by_key(&self, request: &GeoRequest) -> Result<RoutingDecision> {
        if let Some(key) = &request.key {
            if let Some(region_prefix) = key.split(':').next() {
                if let Some(region) = self.config.regions.iter().find(|r| r.id == region_prefix) {
                    return Ok(RoutingDecision {
                        region: region.id.clone(),
                        endpoints: region.endpoints.clone(),
                        reason: format!("Key prefix match: {}", region_prefix),
                    });
                }
            }
        }

        let region = self
            .config
            .default_region
            .clone()
            .unwrap_or_else(|| self.config.local_region.clone());

        Ok(RoutingDecision {
            region,
            endpoints: vec![],
            reason: "Default region".to_string(),
        })
    }

    /// Route based on client's geographic location
    fn route_by_geo(&self, request: &GeoRequest) -> Result<RoutingDecision> {
        let client_location = match &request.client_location {
            Some(loc) => *loc,
            None => {
                if let Some(ip) = &request.client_ip {
                    let cache = self.ip_location_cache.read().unwrap();
                    cache.get(ip).copied().unwrap_or(GeoLocation::new(0.0, 0.0))
                } else {
                    GeoLocation::new(0.0, 0.0)
                }
            }
        };

        let mut nearest = None;
        let mut min_distance = f64::MAX;

        for region in &self.config.regions {
            if let Some(loc) = &region.location {
                let distance = client_location.distance_km(loc);
                if distance < min_distance {
                    min_distance = distance;
                    nearest = Some(region.clone());
                }
            }
        }

        match nearest {
            Some(region) => Ok(RoutingDecision {
                region: region.id.clone(),
                endpoints: region.endpoints.clone(),
                reason: format!("Nearest region: {:.0}km", min_distance),
            }),
            None => Ok(RoutingDecision {
                region: self.config.local_region.clone(),
                endpoints: vec![],
                reason: "No geo data, using local".to_string(),
            }),
        }
    }

    fn route_round_robin(&self, _request: &GeoRequest) -> Result<RoutingDecision> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        let healthy_regions: Vec<_> = self
            .config
            .regions
            .iter()
            .filter(|r| {
                let health = self.region_health.read().unwrap();
                health.get(&r.id).map(|h| h.healthy).unwrap_or(true)
            })
            .collect();

        if healthy_regions.is_empty() {
            return Ok(RoutingDecision {
                region: self.config.local_region.clone(),
                endpoints: vec![],
                reason: "No healthy regions".to_string(),
            });
        }

        let idx = COUNTER.fetch_add(1, Ordering::Relaxed) % healthy_regions.len();
        let region = healthy_regions[idx];

        Ok(RoutingDecision {
            region: region.id.clone(),
            endpoints: region.endpoints.clone(),
            reason: format!("Round-robin index: {}", idx),
        })
    }

    fn route_to_primary(&self, _request: &GeoRequest) -> Result<RoutingDecision> {
        let primary = self
            .config
            .regions
            .iter()
            .find(|r| r.primary)
            .or_else(|| self.config.regions.first());

        match primary {
            Some(region) => Ok(RoutingDecision {
                region: region.id.clone(),
                endpoints: region.endpoints.clone(),
                reason: "Primary region".to_string(),
            }),
            None => Ok(RoutingDecision {
                region: self.config.local_region.clone(),
                endpoints: vec![],
                reason: "No primary defined".to_string(),
            }),
        }
    }

    pub fn check_geo_fence(&self, key: &str, region: &str) -> Result<bool> {
        if !self.config.geo_fencing {
            return Ok(true);
        }

        if key.starts_with("__geo:") {
            let parts: Vec<&str> = key.splitn(3, ':').collect();
            if parts.len() >= 2 {
                let allowed_region = parts[1];
                return Ok(allowed_region == region || allowed_region == "*");
            }
        }

        Ok(true)
    }

    pub fn update_health(
        &self,
        region_id: &str,
        healthy: bool,
        latency_ms: Option<f64>,
        error: Option<String>,
    ) {
        let mut health = self.region_health.write().unwrap();
        let region = self.config.regions.iter().find(|r| r.id == region_id);

        health.insert(
            region_id.to_string(),
            RegionHealth {
                region_id: region_id.to_string(),
