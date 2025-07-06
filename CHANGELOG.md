# Changelog

All notable changes to minikv will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

- No unreleased entries yet.

---

## [1.0.0] - 2026-04-08

### Added - v1.0.0 GA Release

#### Data Science and Engineering Experience
- Notebook-first Python SDK with analytics-oriented client helpers
- End-to-end quickstart example for vector search and time-series workflows
- Dataframe conversion helpers for pandas, polars, and pyarrow

#### Vector Search
- New vector endpoints:
  - `POST /vector/upsert`
  - `POST /vector/query`
  - `GET /admin/vector/stats`
- Cosine similarity top-k matching
- Persistent on-disk vector index snapshot (`coord-data/vector_index.json`)

#### Time-Series API GA
- Production handlers for `POST /ts/write` and `POST/GET /ts/query`
- Automatic initialization of the time-series engine
- Query support for tags, aggregation, resolution, and limits
- Admin time-series stats endpoint now backed by live engine stats

#### Operations and Release Engineering
- Helm chart added with dev/staging/prod values profiles
- Observability bundle:
  - Grafana dashboard provisioning and default minikv overview dashboard
  - Prometheus alert rules for availability and latency/error SLOs
- Backup/restore runbook for release and disaster-recovery drills
- GA preflight release script and Makefile targets for release validation

#### Documentation
- README restructured for GA usage and operations
- CONTRIBUTING guide updated for v1 release workflow
- Release engineering runbook added for reproducible v1.0.0 process

#### Fixes
- Updated shell automation scripts to use Kubernetes-aligned health probes (`/health/live`)

---

## [0.9.0] - 2026-02-14

### Added - v0.9.0 Release

#### Kubernetes Operator
- Custom Resource Definition (CRD) for `MiniKVCluster`
- Automated deployment and scaling of coordinator and volume nodes
- RBAC configuration with proper permissions
- StatefulSet management for persistent storage
- ConfigMap generation from cluster spec
- Horizontal Pod Autoscaler (HPA) support
- ServiceMonitor for Prometheus integration
- Example manifests for basic and production clusters

#### Time-Series Optimizations
- Dedicated `TimeseriesEngine` for time-series workloads
- Multiple resolution levels (raw, 1min, 5min, 1hour, 1day)
- Automatic downsampling with configurable retention
- Aggregation functions: sum, avg, min, max, count, first, last, stddev
- Delta and gorilla compression for efficient storage
- Time-range queries with aggregation support

#### Geo-Partitioning
- Geographic data locality for compliance (GDPR, data residency)
- Multiple routing strategies:
  - Nearest region (latency-based)
  - Primary region (consistency-based)
  - Round-robin (load distribution)
  - Geo-fenced (compliance-based)
- Haversine distance calculation for region selection
- Per-key prefix region assignment
- Region health monitoring with automatic failover
- Geo-fencing rules for data sovereignty

#### Data Tiering
- Automatic data movement between storage tiers
- Tier levels: Hot, Warm, Cold, Archive
- Policy-based tiering rules:
  - Access frequency (access count in time window)
  - Data age (time since creation)
  - Value size (bytes)
  - Last access time (idle duration)
- Access pattern tracking with history
- Compression per tier (none, lz4, zstd, snappy, gzip)
- S3-compatible archive tier support
- Statistics and reporting for tier distribution

#### io_uring Performance Mode (Linux)
- Zero-copy I/O operations using Linux io_uring
- Batched submissions for reduced syscalls
- Configurable submission/completion queue depths
- Optional kernel polling (SQPOLL) for ultra-low latency
- Registered buffers for zero-copy transfers
- Direct I/O bypass of page cache
- Graceful fallback to standard I/O on unsupported systems
- Write batching for small operations

#### New Modules
- `k8s_operator.rs` - Kubernetes controller and CRD types
- `timeseries.rs` - Time-series storage engine
- `geo.rs` - Geo-partitioning router
- `tiering.rs` - Data tiering manager
- `io_uring.rs` - io_uring I/O backend

#### Kubernetes Manifests
- `k8s/crds/minikvcluster.yaml` - CRD definition
- `k8s/rbac/operator-rbac.yaml` - Operator permissions
- `k8s/operator/deployment.yaml` - Operator deployment
- `k8s/examples/basic-cluster.yaml` - Minimal cluster example
- `k8s/examples/production-cluster.yaml` - Production-ready example

---

## [0.8.0] - 2026-02-01

### Added - v0.8.0 Release

#### Cross-Datacenter Replication
- Asynchronous replication to remote datacenters
- Multiple conflict resolution strategies:
  - Last-Write-Wins (timestamp-based)
  - Vector Clocks (causality tracking)
  - Local-First (prefer local DC writes)
  - Primary-First (prefer primary DC writes)
- DC-aware routing for read/write operations
- Configurable replication lag monitoring and alerts
- Per-datacenter replication status tracking

#### Change Data Capture (CDC)
- Real-time capture of all data changes (INSERT, UPDATE, DELETE)
- Configurable sinks for event delivery:
  - Webhook sink (HTTP POST to external endpoints)
