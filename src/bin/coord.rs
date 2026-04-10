//! Coordinator binary

use clap::{Parser, Subcommand};
use minikv::{common::CoordinatorConfig, Coordinator};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Parser)]
#[command(name = "minikv-coord")]
#[command(about = "minikv coordinator with Raft consensus")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Serve {
        #[arg(long)]
        id: String,

        #[arg(long, default_value = "0.0.0.0:8000")]
        bind: String,

        #[arg(long, default_value = "0.0.0.0:8001")]
        grpc: String,

        #[arg(long, default_value = "./coord-data")]
        db: PathBuf,

        #[arg(long, value_delimiter = ',')]
        peers: Vec<String>,

        #[arg(long, default_value = "3")]
        replicas: usize,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Serve {
            id,
            bind,
            grpc,
            db,
            peers,
            replicas,
        } => {
            let config = minikv::common::config::Config::load();
            let bind_addr = bind.parse()?;
            let grpc_addr = grpc.parse()?;
            let db_path = db;
            let mut coord_config = CoordinatorConfig {
                bind_addr,
                grpc_addr,
                db_path,
