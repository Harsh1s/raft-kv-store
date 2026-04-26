//! CLI for cluster operations
//!
//! This module implements the command-line interface for cluster management.
//! Provides commands for verification, repair, and compaction of the distributed key-value store.

use clap::{Parser, Subcommand};
use minikv::ops::{
    auto_rebalance_cluster, compact_cluster, prepare_seamless_upgrade, repair_cluster,
    stream_large_blob, verify_cluster,
};

#[derive(Parser)]
#[command(name = "minikv")]
#[command(about = "minikv distributed key-value store CLI")]
#[command(version)]
struct Cli {
    #[arg(long, default_value = "http://localhost:5000")]
    coordinator: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Verify {
        #[arg(long)]
        deep: bool,

        #[arg(long, default_value = "16")]
        concurrency: usize,
    },

    Repair {
        #[arg(long, default_value = "3")]
        replicas: usize,

        #[arg(long)]
        dry_run: bool,
    },

    Compact {
        #[arg(long)]
        shard: Option<u64>,
    },

    Put {
        key: String,

        #[arg(long)]
