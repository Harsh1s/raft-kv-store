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

