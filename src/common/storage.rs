impl Default for MemStore {
    fn default() -> Self {
        Self::new()
    }
}
///
/// Supports in-memory, RocksDB, and Sled backends. Used for S3/data paths.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[cfg(feature = "rocksdb")]
use rocksdb::{Options, DB};
#[cfg(feature = "sled")]
use sled;

pub trait KVStore: Send + Sync {
    fn get(&self, key: &str) -> Option<Vec<u8>>;
    fn put(&self, key: &str, value: Vec<u8>);
    fn delete(&self, key: &str);
}

pub struct MemStore {
    map: Mutex<HashMap<String, Vec<u8>>>,
}

impl MemStore {
    pub fn new() -> Self {
        Self {
            map: Mutex::new(HashMap::new()),
        }
    }
}

impl KVStore for MemStore {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.map.lock().unwrap().get(key).cloned()
    }
    fn put(&self, key: &str, value: Vec<u8>) {
        self.map.lock().unwrap().insert(key.to_string(), value);
    }
    fn delete(&self, key: &str) {
        self.map.lock().unwrap().remove(key);
    }
}

#[cfg(feature = "rocksdb")]
pub struct RocksStore {
    db: DB,
}

#[cfg(feature = "rocksdb")]
impl RocksStore {
    pub fn new(path: &str) -> Self {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        let db = DB::open(&opts, path).unwrap();
        Self { db }
    }
}

#[cfg(feature = "rocksdb")]
impl KVStore for RocksStore {
    fn get(&self, key: &str) -> Option<Vec<u8>> {
        self.db.get(key).unwrap()
    }
    fn put(&self, key: &str, value: Vec<u8>) {
        self.db.put(key, value).unwrap();
    }
    fn delete(&self, key: &str) {
        self.db.delete(key).unwrap();
    }
}

#[cfg(feature = "sled")]
pub struct SledStore {
    db: sled::Db,
}

#[cfg(feature = "sled")]
impl SledStore {
    pub fn new(path: &str) -> Self {
        let db = sled::open(path).unwrap();
        Self { db }
    }
}

