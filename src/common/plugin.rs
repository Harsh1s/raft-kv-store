//! Plugin system for extending minikv with custom storage, auth, and hooks.

use crate::common::{Error, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PluginVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl PluginVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn is_compatible(&self, other: &PluginVersion) -> bool {
        self.major == other.major
    }
}

impl std::fmt::Display for PluginVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub id: String,

    pub name: String,

    pub description: String,

    pub version: PluginVersion,

    pub author: String,

    pub homepage: Option<String>,

    pub license: Option<String>,

    pub plugin_type: PluginType,

    pub required_version: PluginVersion,

    pub dependencies: Vec<PluginDependency>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginType {
    Storage,
    Auth,
    Hook,
    Middleware,
    Endpoint,
    General,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub min_version: PluginVersion,
    pub optional: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PluginState {
    Loaded,
    Initialized,
    Enabled,
    Disabled,
    Error,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginConfig {
    pub settings: HashMap<String, serde_json::Value>,
}

impl PluginConfig {
    pub fn new() -> Self {
        Self {
            settings: HashMap::new(),
        }
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.settings
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T: Serialize>(&mut self, key: &str, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.settings.insert(key.to_string(), v);
        }
    }
}

pub struct PluginContext {
    pub config: PluginConfig,
    pub shared_data: Arc<RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>>,
    pub logger: PluginLogger,
}

impl PluginContext {
    pub fn new(config: PluginConfig) -> Self {
        Self {
            config,
            shared_data: Arc::new(RwLock::new(HashMap::new())),
            logger: PluginLogger::new("plugin"),
        }
    }

    pub async fn get_shared<T: 'static + Send + Sync + Clone>(&self, key: &str) -> Option<T> {
        self.shared_data
            .read()
            .await
            .get(key)
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
    }

    pub async fn set_shared<T: 'static + Send + Sync>(&self, key: &str, value: T) {
        self.shared_data
            .write()
            .await
            .insert(key.to_string(), Box::new(value));
    }
}

pub struct PluginLogger {
    prefix: String,
}

impl PluginLogger {
    pub fn new(prefix: &str) -> Self {
        Self {
            prefix: prefix.to_string(),
        }
    }

    pub fn info(&self, message: &str) {
        tracing::info!("[{}] {}", self.prefix, message);
    }

    pub fn warn(&self, message: &str) {
        tracing::warn!("[{}] {}", self.prefix, message);
    }

    pub fn error(&self, message: &str) {
        tracing::error!("[{}] {}", self.prefix, message);
    }

    pub fn debug(&self, message: &str) {
        tracing::debug!("[{}] {}", self.prefix, message);
    }
}

#[async_trait]
pub trait Plugin: Send + Sync {
    fn info(&self) -> &PluginInfo;

    async fn initialize(&mut self, ctx: &PluginContext) -> Result<()>;

    async fn enable(&mut self, ctx: &PluginContext) -> Result<()>;

    async fn disable(&mut self, ctx: &PluginContext) -> Result<()>;

    async fn shutdown(&mut self, ctx: &PluginContext) -> Result<()>;

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

#[async_trait]
pub trait StoragePlugin: Plugin {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    async fn put(&self, key: &str, value: Vec<u8>) -> Result<()>;

    async fn delete(&self, key: &str) -> Result<()>;
