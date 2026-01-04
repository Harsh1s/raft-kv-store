//! Transparent encryption at rest using AES-256-GCM.

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use once_cell::sync::Lazy;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::sync::RwLock;

const KEY_SIZE: usize = 32;

const NONCE_SIZE: usize = 12;

const TAG_SIZE: usize = 16;

const ENCRYPTION_MAGIC: &[u8] = b"MKVENC01";

pub static ENCRYPTION_MANAGER: Lazy<RwLock<EncryptionManager>> =
    Lazy::new(|| RwLock::new(EncryptionManager::new()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    pub enabled: bool,
    pub master_key: Option<String>,
    pub key_contexts: Vec<String>,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            master_key: None,
            key_contexts: vec![
                "minikv-data".to_string(),
                "minikv-wal".to_string(),
                "minikv-index".to_string(),
            ],
        }
    }
}

pub type EncryptionResult<T> = std::result::Result<T, EncryptionError>;

#[derive(Debug, Clone)]
pub enum EncryptionError {
    NotEnabled,
    InvalidKey(String),
    EncryptionFailed(String),
    DecryptionFailed(String),
    InvalidFormat(String),
    KeyDerivationFailed(String),
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EncryptionError::NotEnabled => write!(f, "Encryption is not enabled"),
            EncryptionError::InvalidKey(msg) => write!(f, "Invalid encryption key: {}", msg),
            EncryptionError::EncryptionFailed(msg) => write!(f, "Encryption failed: {}", msg),
            EncryptionError::DecryptionFailed(msg) => write!(f, "Decryption failed: {}", msg),
            EncryptionError::InvalidFormat(msg) => {
                write!(f, "Invalid encrypted data format: {}", msg)
            }
            EncryptionError::KeyDerivationFailed(msg) => {
                write!(f, "Key derivation failed: {}", msg)
            }
        }
    }
}

impl std::error::Error for EncryptionError {}

#[derive(Debug, Clone)]
pub struct EncryptedData {
    pub nonce: [u8; NONCE_SIZE],
    pub ciphertext: Vec<u8>,
}

impl EncryptedData {
    /// Serialize to bytes: MAGIC || NONCE || CIPHERTEXT
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes =
            Vec::with_capacity(ENCRYPTION_MAGIC.len() + NONCE_SIZE + self.ciphertext.len());
        bytes.extend_from_slice(ENCRYPTION_MAGIC);
        bytes.extend_from_slice(&self.nonce);
        bytes.extend_from_slice(&self.ciphertext);
        bytes
    }

    pub fn from_bytes(bytes: &[u8]) -> EncryptionResult<Self> {
        let min_size = ENCRYPTION_MAGIC.len() + NONCE_SIZE + TAG_SIZE;
        if bytes.len() < min_size {
            return Err(EncryptionError::InvalidFormat(format!(
                "Data too short: {} bytes, minimum {} bytes",
                bytes.len(),
                min_size
            )));
        }

        if &bytes[..ENCRYPTION_MAGIC.len()] != ENCRYPTION_MAGIC {
            return Err(EncryptionError::InvalidFormat(
                "Invalid magic bytes - data may not be encrypted".to_string(),
            ));
        }

        let nonce_start = ENCRYPTION_MAGIC.len();
        let ciphertext_start = nonce_start + NONCE_SIZE;

        let mut nonce = [0u8; NONCE_SIZE];
        nonce.copy_from_slice(&bytes[nonce_start..ciphertext_start]);

        let ciphertext = bytes[ciphertext_start..].to_vec();

        Ok(Self { nonce, ciphertext })
    }

    pub fn is_encrypted(bytes: &[u8]) -> bool {
        bytes.len() >= ENCRYPTION_MAGIC.len()
            && &bytes[..ENCRYPTION_MAGIC.len()] == ENCRYPTION_MAGIC
    }
}

pub struct EncryptionManager {
    config: EncryptionConfig,
    data_key: Option<[u8; KEY_SIZE]>,
    wal_key: Option<[u8; KEY_SIZE]>,
    data_cipher: Option<Aes256Gcm>,
    wal_cipher: Option<Aes256Gcm>,
}

impl EncryptionManager {
    pub fn new() -> Self {
        Self {
            config: EncryptionConfig::default(),
            data_key: None,
            wal_key: None,
            data_cipher: None,
            wal_cipher: None,
        }
    }

    pub fn initialize(&mut self, master_key: &str) -> EncryptionResult<()> {
        let key_bytes = BASE64
            .decode(master_key)
            .map_err(|e| EncryptionError::InvalidKey(format!("Invalid base64: {}", e)))?;

        if key_bytes.len() < 32 {
