//! Change Data Capture - captures data changes and streams them to sinks.

use crate::common::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDCEvent {
    pub id: String,

    pub sequence: u64,

    pub timestamp: DateTime<Utc>,

    pub operation: CDCOperation,

    pub key: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_value: Option<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_value: Option<Vec<u8>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,

    #[serde(default)]
    pub metadata: CDCMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum CDCOperation {
    Insert,
    Update,
    Delete,
    Snapshot,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CDCMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_node: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacenter: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub transaction_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor: Option<String>,

    #[serde(default)]
    pub tags: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CDCConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,

    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    #[serde(default = "default_flush_interval")]
    pub flush_interval_ms: u64,

    #[serde(default)]
    pub sinks: Vec<SinkConfig>,

    #[serde(default)]
    pub filter_operations: Vec<CDCOperation>,

    #[serde(default)]
    pub filter_key_prefix: Option<String>,

    #[serde(default = "default_true")]
    pub include_old_values: bool,
}

fn default_true() -> bool {
    true
}

fn default_buffer_size() -> usize {
    10000
}

fn default_batch_size() -> usize {
    100
}

fn default_flush_interval() -> u64 {
    1000
}

impl Default for CDCConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            buffer_size: 10000,
            batch_size: 100,
            flush_interval_ms: 1000,
            sinks: vec![],
            filter_operations: vec![],
            filter_key_prefix: None,
            include_old_values: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum SinkConfig {
    Webhook {
        url: String,
        #[serde(default)]
        headers: std::collections::HashMap<String, String>,
        #[serde(default = "default_timeout")]
        timeout_ms: u64,
        #[serde(default = "default_retries")]
        max_retries: u32,
    },

    Kafka {
        brokers: Vec<String>,
        topic: String,
        #[serde(default)]
        client_id: Option<String>,
    },

    File {
        path: String,
        #[serde(default)]
        rotate_size_mb: Option<u64>,
        #[serde(default)]
        max_files: Option<u32>,
    },

    Memory {
        #[serde(default = "default_memory_limit")]
        max_events: usize,
    },
}

fn default_timeout() -> u64 {
    5000
}

fn default_retries() -> u32 {
    3
}

fn default_memory_limit() -> usize {
    1000
}

#[async_trait]
pub trait CDCSink: Send + Sync {
    fn name(&self) -> &str;

    async fn send(&self, events: Vec<CDCEvent>) -> Result<()>;

    async fn health_check(&self) -> Result<bool>;
}

pub struct WebhookSink {
    name: String,
    url: String,
    headers: std::collections::HashMap<String, String>,
    timeout_ms: u64,
    max_retries: u32,
    client: reqwest::Client,
}

impl WebhookSink {
    pub fn new(
        url: String,
        headers: std::collections::HashMap<String, String>,
        timeout_ms: u64,
        max_retries: u32,
    ) -> Self {
        Self {
            name: format!("webhook:{}", url),
            url,
            headers,
            timeout_ms,
            max_retries,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl CDCSink for WebhookSink {
    fn name(&self) -> &str {
        &self.name
    }

    async fn send(&self, events: Vec<CDCEvent>) -> Result<()> {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            let body = serde_json::to_string(&events)
                .map_err(|e| Error::Other(format!("Failed to serialize events: {}", e)))?;
            let mut request = self
                .client
                .post(&self.url)
