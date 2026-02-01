# Professional Test Scenarios - minikv v1.0.0

This document defines manual validation scenarios for minikv v1.0.0.
Each scenario includes context, steps, and success criteria.

## 0. Kubernetes Operator and Cloud-Native Deployment

Context: Validate CRD lifecycle, reconciliation, RBAC, StatefulSet behavior, and scaling.

Steps:
1. Deploy CRD and operator manifests.
2. Apply basic and production cluster examples.
3. Verify StatefulSet, Services, ConfigMaps, RBAC, and monitoring resources.
4. Scale up and down.
5. Delete and recreate the cluster resource.

Success criteria:
- Resources are created and reconciled correctly.
- Scaling works without orphaned resources.
- RBAC is enforced.
- Cluster returns to healthy state after recreation.

## 1. Time-Series Engine

Context: Validate ingest and query workflows.

Steps:
1. Start coordinator and volumes.
2. Write samples with `POST /ts/write`.
3. Query with `POST /ts/query` using filters and time windows.
4. Verify aggregation behavior.

Success criteria:
- Samples are persisted and queryable.
- Aggregation and filters return expected results.

## 2. Vector Similarity Search

Context: Validate vector indexing and nearest-neighbor retrieval.

Steps:
1. Upsert vectors with `POST /vector/upsert`.
2. Query neighbors with `POST /vector/query`.
3. Check index stats with `GET /admin/vector/stats`.
4. Restart coordinator and verify results again.

Success criteria:
- Upserted vectors are returned by similarity queries.
- `top_k` behavior is respected.
- Index remains usable after restart.

## 3. Geo-Partitioning

Context: Validate routing across regions.

Steps:
1. Configure multiple regions.
2. Test latency, geo, round-robin, and primary routing strategies.
3. Simulate a regional outage and verify fallback.
4. Validate geo-fencing where configured.

Success criteria:
- Requests follow configured strategy.
- Failover is deterministic and safe.
- Geo-fencing rules are respected.

## 4. Data Tiering

Context: Validate movement across hot, warm, cold, and archive tiers.

Steps:
1. Start with tiering enabled.
2. Insert data with varied access patterns.
3. Trigger or wait for policy evaluation.
