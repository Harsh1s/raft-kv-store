//! Authentication and authorization with API keys, JWT, and RBAC.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use once_cell::sync::Lazy;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const DEFAULT_JWT_SECRET: &[u8] = b"minikv-default-secret-change-in-production";

const API_KEY_PREFIX: &str = "mkv_";

const API_KEY_LENGTH: usize = 32;

const JWT_EXPIRATION_HOURS: u64 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Role {
    Admin,
    ReadWrite,
    #[default]
    ReadOnly,
}

impl Role {
    pub fn can_write(&self) -> bool {
        matches!(self, Role::Admin | Role::ReadWrite)
    }

    pub fn can_admin(&self) -> bool {
        matches!(self, Role::Admin)
    }

    pub fn can_read(&self) -> bool {
        true // All roles can read
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub tenant: String,
    pub role: Role,
    pub created_at: u64,
    /// Expiration timestamp (Unix epoch ms), None = never expires
    pub expires_at: Option<u64>,
    pub active: bool,
    pub last_used_at: Option<u64>,
}

impl ApiKey {
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            now >= expires_at
        } else {
            false
        }
    }

    pub fn is_valid(&self) -> bool {
        self.active && !self.is_expired()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub tenant: String,
    pub role: Role,
    pub exp: u64,
    pub iat: u64,
}

#[derive(Debug, Clone)]
pub struct AuthContext {
    pub key_id: String,
    pub tenant: String,
    pub role: Role,
}

impl AuthContext {
    pub fn can_write(&self) -> bool {
        self.role.can_write()
    }

    pub fn can_admin(&self) -> bool {
        self.role.can_admin()
    }
}

#[derive(Debug)]
pub enum AuthResult {
    Ok(AuthContext),
    Missing,
    Invalid(String),
    Expired,
    Forbidden(String),
}

pub struct KeyStore {
    keys: RwLock<HashMap<String, ApiKey>>,
    hash_to_id: RwLock<HashMap<String, String>>,
    jwt_encoding_key: EncodingKey,
    jwt_decoding_key: DecodingKey,
    argon2: Argon2<'static>,
}

impl KeyStore {
    pub fn new() -> Self {
        Self::with_secret(DEFAULT_JWT_SECRET)
    }

    pub fn with_secret(secret: &[u8]) -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
            hash_to_id: RwLock::new(HashMap::new()),
            jwt_encoding_key: EncodingKey::from_secret(secret),
            jwt_decoding_key: DecodingKey::from_secret(secret),
            argon2: Argon2::default(),
        }
    }

    pub fn generate_key(
        &self,
        name: &str,
        tenant: &str,
        role: Role,
        expires_in: Option<Duration>,
    ) -> Result<(String, String), AuthError> {
        let mut rng = rand::thread_rng();
        let random_bytes: [u8; API_KEY_LENGTH] = rng.gen();
        let key_suffix = URL_SAFE_NO_PAD.encode(random_bytes);
        let plaintext_key = format!("{}{}", API_KEY_PREFIX, key_suffix);

        let key_id = uuid::Uuid::new_v4().to_string();

        let salt = SaltString::generate(&mut OsRng);
        let key_hash = self
            .argon2
            .hash_password(plaintext_key.as_bytes(), &salt)
            .map_err(|e| AuthError::HashError(e.to_string()))?
            .to_string();

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let expires_at = expires_in.map(|d| now + d.as_millis() as u64);

        let api_key = ApiKey {
            id: key_id.clone(),
            name: name.to_string(),
            key_hash: key_hash.clone(),
            tenant: tenant.to_string(),
            role,
            created_at: now,
            expires_at,
            active: true,
            last_used_at: None,
        };

        {
            let mut keys = self.keys.write().unwrap();
            keys.insert(key_id.clone(), api_key);
        }
        {
            let mut hash_to_id = self.hash_to_id.write().unwrap();
            hash_to_id.insert(key_hash, key_id.clone());
        }

        Ok((key_id, plaintext_key))
    }

    pub fn validate_key(&self, key: &str) -> AuthResult {
        if !key.starts_with(API_KEY_PREFIX) {
            return AuthResult::Invalid("Invalid key format".to_string());
        }

        let keys = self.keys.read().unwrap();
        for api_key in keys.values() {
            if let Ok(parsed_hash) = PasswordHash::new(&api_key.key_hash) {
                if self
                    .argon2
                    .verify_password(key.as_bytes(), &parsed_hash)
                    .is_ok()
                {
                    if !api_key.active {
                        return AuthResult::Invalid("Key is disabled".to_string());
                    }
                    if api_key.is_expired() {
                        return AuthResult::Expired;
                    }

                    return AuthResult::Ok(AuthContext {
                        key_id: api_key.id.clone(),
                        tenant: api_key.tenant.clone(),
                        role: api_key.role,
                    });
                }
            }
        }

        AuthResult::Invalid("Invalid API key".to_string())
    }

    pub fn generate_jwt(&self, auth: &AuthContext) -> Result<String, AuthError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: auth.key_id.clone(),
            tenant: auth.tenant.clone(),
            role: auth.role,
            exp: now + (JWT_EXPIRATION_HOURS * 3600),
            iat: now,
        };

        encode(&Header::default(), &claims, &self.jwt_encoding_key)
            .map_err(|e| AuthError::JwtError(e.to_string()))
    }

    pub fn validate_jwt(&self, token: &str) -> AuthResult {
        let validation = Validation::default();

        match decode::<Claims>(token, &self.jwt_decoding_key, &validation) {
            Ok(token_data) => {
                let claims = token_data.claims;

                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                if now >= claims.exp {
                    return AuthResult::Expired;
                }

                AuthResult::Ok(AuthContext {
                    key_id: claims.sub,
                    tenant: claims.tenant,
                    role: claims.role,
                })
            }
            Err(e) => AuthResult::Invalid(format!("Invalid JWT: {}", e)),
        }
    }

    pub fn authenticate(&self, auth_header: &str) -> AuthResult {
        let parts: Vec<&str> = auth_header.splitn(2, ' ').collect();
        if parts.len() != 2 {
            return AuthResult::Invalid("Invalid Authorization header format".to_string());
        }

        match parts[0].to_lowercase().as_str() {
            "bearer" => self.validate_jwt(parts[1]),
            "apikey" => self.validate_key(parts[1]),
            _ => AuthResult::Invalid(format!("Unknown auth scheme: {}", parts[0])),
        }
    }

    pub fn get_key(&self, key_id: &str) -> Option<ApiKey> {
        let keys = self.keys.read().unwrap();
        keys.get(key_id).cloned()
    }

    pub fn list_keys(&self) -> Vec<ApiKey> {
        let keys = self.keys.read().unwrap();
        keys.values().cloned().collect()
    }

    pub fn list_keys_for_tenant(&self, tenant: &str) -> Vec<ApiKey> {
        let keys = self.keys.read().unwrap();
        keys.values()
            .filter(|k| k.tenant == tenant)
            .cloned()
            .collect()
    }

    pub fn revoke_key(&self, key_id: &str) -> Result<(), AuthError> {
        let mut keys = self.keys.write().unwrap();
        if let Some(key) = keys.get_mut(key_id) {
            key.active = false;
            Ok(())
        } else {
            Err(AuthError::KeyNotFound(key_id.to_string()))
        }
    }

    pub fn delete_key(&self, key_id: &str) -> Result<(), AuthError> {
        let mut keys = self.keys.write().unwrap();
        if let Some(key) = keys.remove(key_id) {
            let mut hash_to_id = self.hash_to_id.write().unwrap();
            hash_to_id.remove(&key.key_hash);
            Ok(())
        } else {
            Err(AuthError::KeyNotFound(key_id.to_string()))
        }
    }

    pub fn touch_key(&self, key_id: &str) {
        let mut keys = self.keys.write().unwrap();
        if let Some(key) = keys.get_mut(key_id) {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64;
            key.last_used_at = Some(now);
        }
    }
}

