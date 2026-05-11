//! # minikv
//!
//! A production-grade distributed key-value store with:
//! - Raft consensus for coordinator high availability
//! - Write-ahead log (WAL) for durability
//! - Automatic compaction and rebalancing
//! - gRPC for internal coordination, HTTP for public API
//! - Bloom filters and index snapshots for performance
//!
//! ## Architecture

#![allow(clippy::result_large_err)]
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │         Coordinator Cluster             │
//! │  (Raft consensus for metadata)          │
//! │   - Leader: handles writes              │
//! │   - Followers: replicate state          │
//! └───────────┬─────────────────────────────┘
//!             │ gRPC
//!   ┌─────────┴──────────┬──────────────┐
//!   │                    │              │
//! ┌─▼─────────┐   ┌─────▼──────┐   ┌──▼───────────┐
//! │ Volume 1   │   │ Volume 2   │   │ Volume 3     │
//! │ (Shard A)  │   │ (Shard B)  │   │ (Shard C)    │
//! │  + WAL     │   │  + WAL     │   │  + WAL       │
