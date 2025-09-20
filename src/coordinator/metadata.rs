use once_cell::sync::OnceCell;
use std::sync::Arc;
static GLOBAL_STORE: OnceCell<Arc<MetadataStore>> = OnceCell::new();

pub fn init_global_store(store: MetadataStore) {
    let _ = GLOBAL_STORE.set(Arc::new(store));
}

pub fn get_global_store() -> Arc<MetadataStore> {
    GLOBAL_STORE
        .get()
        .expect("Global MetadataStore not initialized")
        .clone()
}
///
/// It stores key metadata (replicas, size, hash, timestamps), volume registry (node_id to address, state, shards), and cluster configuration.
use crate::common::{NodeState, Result};
use rocksdb::{Options, DB};
use serde::{Deserialize, Serialize};
use std::path::Path;

const CF_KEYS: &str = "keys";
const CF_VOLUMES: &str = "volumes";
const CF_CONFIG: &str = "config";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMetadata {
    pub key: String,
    pub replicas: Vec<String>,
    pub size: u64,
    pub blake3: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub state: KeyState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyState {
    Active,
    Tombstone,
}

/// Describes a single volume in the cluster, including its address, state, and assigned shards.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeMetadata {
    pub volume_id: String,
    pub address: String,
    pub grpc_address: String,
    pub state: NodeState,
    pub shards: Vec<u64>,
    pub total_keys: u64,
    pub total_bytes: u64,
    pub free_bytes: u64,
    pub last_heartbeat: u64,
}

pub struct MetadataStore {
    db: DB,
}

impl MetadataStore {
    #[allow(clippy::result_large_err)]
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let db = DB::open_cf(&opts, path, vec![CF_KEYS, CF_VOLUMES, CF_CONFIG])?;

        Ok(Self { db })
    }

    #[allow(clippy::result_large_err)]
    pub fn put_key(&self, meta: &KeyMetadata) -> Result<()> {
        let cf = self.db.cf_handle(CF_KEYS).unwrap();
        let value = bincode::serialize(meta)
