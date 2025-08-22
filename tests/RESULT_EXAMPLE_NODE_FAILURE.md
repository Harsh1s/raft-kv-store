# Professional Test Report - Example

Scenario: Node Failure (Manual Validation)
Template version: minikv v1.0.0

## General Information

- Date: 2026-04-08
- Tester: Em
- Scenario ID and Name: 6 - Node Failure
- Build/Version (`git rev-parse --short HEAD`): local working tree
- Environment (local, Docker, k8s): Docker Compose (local)
- Cluster configuration (coordinator, volumes, replicas): 1 coordinator, 3 volumes, replication factor 3

## Objective

- Objective: Validate cluster availability and recovery when one volume becomes unavailable.
- Scope: Health endpoints, read/write behavior during outage, and post-restart recovery.
- Preconditions:
  - Cluster started and healthy.
  - Initial dataset written successfully.
  - Metrics endpoint reachable.

## Steps Executed

1. Started cluster with coordinator + 3 volumes.
2. Inserted baseline dataset (1000 keys).
3. Stopped one volume container (`minikv-volume-1`).
4. Checked `GET /health/live`, `GET /health/ready`, and `GET /metrics`.
5. Executed read/write requests while one volume was down.
6. Restarted stopped volume.
7. Re-checked health, metrics, and data consistency.

## Commands Used

```bash
# Start stack
docker compose up -d

# Baseline writes (example)
for i in $(seq 1 1000); do
  curl -s -X PUT "http://localhost:8080/s3/test-bucket/key-$i" \
    --data "value-$i" >/dev/null
 done

# Stop one volume
docker stop minikv-volume-1

# Health and metrics checks
curl -s http://localhost:8080/health/live
curl -s http://localhost:8080/health/ready
curl -s http://localhost:8080/metrics | head -n 40

# Sample reads during outage
