#!/usr/bin/env bash
# Convenience script to start a local minikv cluster

set -euo pipefail

NUM_COORDS="${1:-3}"
NUM_VOLUMES="${2:-3}"

echo "Starting minikv cluster"
echo "  Coordinators: ${NUM_COORDS}"
echo "  Volumes: ${NUM_VOLUMES}"
echo ""

# Build first
echo "Building..."
cargo build --release

# Create data directories
mkdir -p data/coord{1..${NUM_COORDS}}
mkdir -p data/vol{1..${NUM_VOLUMES}}-{data,wal}

# Start coordinators
echo "Starting coordinators..."
for i in $(seq 1 ${NUM_COORDS}); do
    COORD_HTTP=$((5000 + (i-1)*2))
    COORD_GRPC=$((5001 + (i-1)*2))
    
