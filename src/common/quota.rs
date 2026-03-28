//! Tenant-based resource quotas (storage, objects, rate limits).

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

const DEFAULT_STORAGE_LIMIT: u64 = 10 * 1024 * 1024 * 1024; // 10 GB
const DEFAULT_OBJECT_LIMIT: u64 = 1_000_000; // 1 million objects
const DEFAULT_RATE_LIMIT: u32 = 1000; // requests per second
const DEFAULT_RATE_WINDOW: Duration = Duration::from_secs(1);

pub static QUOTA_MANAGER: Lazy<QuotaManager> = Lazy::new(QuotaManager::new);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantQuota {
    pub tenant_id: String,
    /// Maximum storage in bytes (0 = unlimited)
    pub storage_limit: u64,
    /// Maximum number of objects (0 = unlimited)
    pub object_limit: u64,
    /// Maximum requests per rate window (0 = unlimited)
    pub rate_limit: u32,
    pub enabled: bool,
    #[serde(skip)]
    pub created_at: Option<Instant>,
}

impl TenantQuota {
    pub fn new(tenant_id: String) -> Self {
        Self {
            tenant_id,
            storage_limit: DEFAULT_STORAGE_LIMIT,
            object_limit: DEFAULT_OBJECT_LIMIT,
            rate_limit: DEFAULT_RATE_LIMIT,
            enabled: true,
            created_at: Some(Instant::now()),
        }
    }

    pub fn unlimited(tenant_id: String) -> Self {
        Self {
            tenant_id,
            storage_limit: 0,
            object_limit: 0,
            rate_limit: 0,
            enabled: true,
            created_at: Some(Instant::now()),
        }
    }

    pub fn with_limits(
        tenant_id: String,
        storage_limit: u64,
        object_limit: u64,
        rate_limit: u32,
    ) -> Self {
        Self {
            tenant_id,
            storage_limit,
            object_limit,
            rate_limit,
            enabled: true,
            created_at: Some(Instant::now()),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TenantUsage {
    pub storage_used: u64,
    pub object_count: u64,
    pub request_times: Vec<Instant>,
}

impl TenantUsage {
    pub fn check_storage(&self, quota: &TenantQuota, additional_bytes: u64) -> bool {
        if quota.storage_limit == 0 {
            return true; // Unlimited
        }
        self.storage_used + additional_bytes <= quota.storage_limit
    }

    pub fn check_objects(&self, quota: &TenantQuota, additional_objects: u64) -> bool {
        if quota.object_limit == 0 {
            return true; // Unlimited
        }
        self.object_count + additional_objects <= quota.object_limit
    }

    pub fn check_rate(&mut self, quota: &TenantQuota) -> bool {
        if quota.rate_limit == 0 {
            return true; // Unlimited
        }

        let now = Instant::now();
        let window_start = now - DEFAULT_RATE_WINDOW;

        self.request_times.retain(|&t| t > window_start);

        (self.request_times.len() as u32) < quota.rate_limit
    }

    pub fn record_request(&mut self) {
        self.request_times.push(Instant::now());
    }

    pub fn add_storage(&mut self, bytes: u64) {
        self.storage_used = self.storage_used.saturating_add(bytes);
    }

    pub fn remove_storage(&mut self, bytes: u64) {
        self.storage_used = self.storage_used.saturating_sub(bytes);
    }

    pub fn add_objects(&mut self, count: u64) {
        self.object_count = self.object_count.saturating_add(count);
    }

    pub fn remove_objects(&mut self, count: u64) {
        self.object_count = self.object_count.saturating_sub(count);
    }
}

#[derive(Debug, Clone)]
pub enum QuotaCheckResult {
    Allowed,
    TenantNotFound,
    StorageLimitExceeded {
        limit: u64,
        used: u64,
        requested: u64,
    },
    ObjectLimitExceeded {
        limit: u64,
        count: u64,
    },
    RateLimitExceeded {
        limit: u32,
        window_secs: u64,
    },
