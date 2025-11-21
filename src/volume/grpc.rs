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
