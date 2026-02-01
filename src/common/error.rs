//!  Error types for minikv

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Corrupted data: {0}")]
    Corrupted(String),

    #[error("Checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    #[error("WAL error: {0}")]
    Wal(String),

    #[error("Not leader: current leader is {0}")]
    NotLeader(String),

    #[error("Raft error: {0}")]
    Raft(String),

    #[error("Consensus timeout")]
    ConsensusTimeout,

    #[error("Prepare failed on {node}: {reason}")]
    PrepareFailed { node: String, reason: String },

    #[error("Commit failed on {node}: {reason}")]
    CommitFailed { node: String, reason: String },

    #[error("No healthy volumes available")]
    NoHealthyVolumes,

    #[error("Insufficient replicas: need {needed}, have {available}")]
    InsufficientReplicas { needed: usize, available: usize },

    #[error("Shard not found: {0}")]
    ShardNotFound(u64),

    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("RocksDB error: {0}")]
    RocksDb(#[from] rocksdb::Error),

    #[error("Metadata corrupted: {0}")]
    MetadataCorrupted(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Verify failed: {0}")]
    VerifyFailed(String),

    #[error("Repair failed: {0}")]
    RepairFailed(String),

    #[error("Compact failed: {0}")]
    CompactFailed(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Operation timeout: {0}")]
    Timeout(String),

    #[error("{0}")]
    Other(String),
}

impl Error {
    /// Is this a retryable error?
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Error::Timeout(_)
                | Error::ConnectionFailed(_)
                | Error::ConsensusTimeout
                | Error::NotLeader(_)
                | Error::NoHealthyVolumes
        )
    }

    pub fn to_grpc_status(&self) -> tonic::Status {
        use tonic::Code;
        match self {
