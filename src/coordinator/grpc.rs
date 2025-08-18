//! Coordinator gRPC service (internal)
//!
//! This module exposes the internal gRPC API for cluster coordination.
//! Used for Raft consensus, metadata replication, and distributed operations between nodes.

use crate::proto::coordinator_internal_server::{CoordinatorInternal, CoordinatorInternalServer};
use crate::proto::*;
use tonic::{Request, Response, Status};

/// CoordGrpcService implements the internal gRPC API for cluster coordination.
pub struct CoordGrpcService {}

impl Default for CoordGrpcService {
    fn default() -> Self {
        Self::new()
    }
}

impl CoordGrpcService {
    pub fn new() -> Self {
        Self {}
    }

    pub fn into_server(self) -> CoordinatorInternalServer<Self> {
        CoordinatorInternalServer::new(self)
    }
}

#[tonic::async_trait]
impl CoordinatorInternal for CoordGrpcService {
    async fn range(
        &self,
        req: Request<crate::proto::RangeRequest>,
    ) -> Result<Response<crate::proto::RangeResponse>, Status> {
        let store = crate::coordinator::metadata::get_global_store();
        let params = req.into_inner();
        let keys = match store.list_keys() {
            Ok(keys) => keys,
            Err(e) => return Err(Status::internal(format!("list_keys error: {}", e))),
        };
        let mut filtered: Vec<String> = keys
            .into_iter()
            .filter(|k| k >= &params.start && k <= &params.end)
            .collect();
        filtered.sort();
        let mut values = Vec::new();
        if params.include_values {
            for k in &filtered {
                match store.get_key(k) {
                    Ok(Some(meta)) => values.push(bincode::serialize(&meta).unwrap_or_default()),
                    _ => values.push(vec![]),
                }
            }
        }
