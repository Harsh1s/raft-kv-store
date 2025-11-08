use axum_server::tls_rustls::{bind_rustls, RustlsConfig};
use std::future::IntoFuture;

use crate::common::{CoordinatorConfig, Result};
use crate::coordinator::grpc::CoordGrpcService;
use crate::coordinator::http::{create_router, CoordState};
use crate::coordinator::metadata::MetadataStore;
use crate::coordinator::placement::PlacementManager;
use crate::coordinator::raft_node::{start_raft_tasks, RaftNode};
use std::sync::{Arc, Mutex};

pub struct Coordinator {
    config: CoordinatorConfig,
    node_id: String,
}

impl Coordinator {
    pub fn new(config: CoordinatorConfig, node_id: String) -> Self {
        Self { config, node_id }
    }

    pub async fn serve(self) -> Result<()> {
        tracing::info!("Starting coordinator: {}", self.node_id);
        tracing::info!("  HTTP API: {}", self.config.bind_addr);
        tracing::info!("  gRPC API: {}", self.config.grpc_addr);
        tracing::info!("  DB path: {}", self.config.db_path.display());
        tracing::info!("  Replicas: {}", self.config.replicas);

        let metadata = Arc::new(MetadataStore::open(&self.config.db_path)?);

        let placement = Arc::new(Mutex::new(PlacementManager::new(
            self.config.num_shards,
            self.config.replicas,
        )));
