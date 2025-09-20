//! Volume gRPC service implementation
//!
//! This module exposes the internal gRPC API for volume operations.

use crate::proto::volume_internal_server::{VolumeInternal, VolumeInternalServer};
use crate::proto::*;
use crate::volume::blob::BlobStore;
use std::sync::{Arc, Mutex};
use tonic::{Request, Response, Status};

pub struct VolumeGrpcService {
    store: Arc<Mutex<BlobStore>>,
}

impl VolumeGrpcService {
    pub fn new(store: BlobStore) -> Self {
        VolumeGrpcService {
            store: Arc::new(Mutex::new(store)),
        }
    }

    pub fn into_server(self) -> VolumeInternalServer<Self> {
        VolumeInternalServer::new(self)
    }
}

#[tonic::async_trait]
impl VolumeInternal for VolumeGrpcService {
    async fn prepare(
        &self,
        req: Request<PrepareRequest>,
    ) -> Result<Response<PrepareResponse>, Status> {
        let inner = req.into_inner();

        if inner.key.is_empty() {
            return Ok(Response::new(PrepareResponse {
                ok: false,
                error: "key cannot be empty".to_string(),
            }));
        }

        Ok(Response::new(PrepareResponse {
            ok: true,
            error: String::new(),
        }))
    }

    async fn commit(
        &self,
        req: Request<CommitRequest>,
    ) -> Result<Response<CommitResponse>, Status> {
        let _inner = req.into_inner();

        Ok(Response::new(CommitResponse {
            ok: true,
            error: String::new(),
        }))
    }

    async fn abort(&self, req: Request<AbortRequest>) -> Result<Response<AbortResponse>, Status> {
        let _inner = req.into_inner();

        Ok(Response::new(AbortResponse { ok: true }))
    }

    async fn pull(&self, _req: Request<PullRequest>) -> Result<Response<Self::PullStream>, Status> {
        Err(Status::unimplemented("Pull not implemented"))
    }

    async fn delete(
