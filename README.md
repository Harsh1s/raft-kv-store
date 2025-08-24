# minikv

Distributed, multi-tenant key-value and object store in Rust, with Raft consensus, WAL durability, and production-oriented operations.

[![Repo](https://img.shields.io/badge/github-whispem%2Fminikv-blue)](https://github.com/whispem/minikv)
[![Rust](https://img.shields.io/badge/rust-1.81+-orange.svg)](https://rustup.rs/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](.github/workflows/ci.yml)

## v1.0.0

v1.0.0 is the GA line focused on data/ML workflows and operational reliability.

- Time-series API handlers are production-backed (`/ts/write`, `/ts/query`).
- Vector search endpoints are available and persisted on coordinator disk.
- Python SDK preview supports notebook workflows (pandas/polars/pyarrow helpers).
- Observability bundle now includes Grafana dashboard provisioning and Prometheus alerts.
- Helm chart is available with dev/staging/prod values profiles.
- Release engineering includes preflight checks and runbooks.

## Table of Contents

- [What is minikv](#what-is-minikv)
- [Quick Start](#quick-start)
- [Python SDK](#python-sdk)
- [Core Features](#core-features)
- [Operations and Release Engineering](#operations-and-release-engineering)
- [Roadmap](#roadmap)
- [Development](#development)
- [Contributing](#contributing)

## What is minikv

minikv is a distributed systems reference implementation and an extensible data platform.

- Strong consistency: Raft for metadata and 2PC patterns for distributed writes.
- Durability: WAL and pluggable storage backends.
- Multi-tenancy and security: RBAC, API keys/JWT, encryption at rest.
- Real-time and analytics pathways: watch/SSE and time-series APIs.

## Quick Start

Build and run local cluster:

```bash
git clone https://github.com/whispem/minikv.git
cd minikv
make build
make serve
```

Basic checks:

```bash
curl -s http://127.0.0.1:5000/health/live
curl -s http://127.0.0.1:5000/health/ready
curl -s http://127.0.0.1:5000/metrics
curl -s http://127.0.0.1:5000/admin/status
```

## Python SDK

Notebook-first SDK (preview) for data scientists and engineers:

- Time-series write/query helpers
- Vector upsert/query helpers
- SSE change-stream consumption
- Dataframe conversions (pandas, polars, pyarrow)

Install and run example:

```bash
pip install -r sdk/python/requirements.txt
python examples/data_science_quickstart.py
```

Files:

- `sdk/python/minikv_client.py`
- `sdk/python/README.md`
- `examples/data_science_quickstart.py`

## Core Features

Distribution and consistency:

- Raft consensus (leader election, log replication)
- 256 virtual shards and placement management
- Multi-key operations and transaction endpoints
- Cross-DC replication primitives and conflict policies

Storage and query paths:

- Pluggable backends: RocksDB, Sled, in-memory
- WAL, compaction, and integrity tooling
- Time-series engine with aggregation and downsampling
- Vector similarity endpoints with cosine top-k search

Security and tenancy:

- API keys (Argon2), JWT, RBAC
- AES-256-GCM encryption at rest
- Tenant quotas and request rate limiting
- Audit logging

APIs:

- HTTP REST and S3-compatible endpoints
- WebSocket/SSE watch endpoints
- gRPC internal communication

## Operations and Release Engineering

Observability:

- Prometheus and OpenTelemetry integration
- Grafana dashboard provisioning
- Alert rules in `opentelemetry/prometheus-alerts.yml`

Kubernetes:

- Operator manifests under `k8s/`
- Helm chart under `k8s/helm/minikv/`

Runbooks:

