#!/usr/bin/env bash
# Comprehensive benchmark for minikv distributed cluster

set -euo pipefail

# Configuration
NUM_COORDS="${1:-3}"
NUM_VOLUMES="${2:-3}"
REPLICAS="${3:-3}"
VUS="${4:-16}"
DURATION="${5:-30s}"
OBJECT_SIZE="${6:-1048576}"  # 1 MB

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_banner() {
    echo -e "${BLUE}================================${NC}"
    echo -e "${BLUE}  minikv Benchmark${NC}"
    echo -e "${BLUE}================================${NC}"
}

check_deps() {
    local missing=()
    
    if ! command -v cargo &> /dev/null; then
        missing+=("cargo")
    fi
    
    if ! command -v k6 &> /dev/null; then
        missing+=("k6 (brew install k6)")
    fi
    
    if ! command -v jq &> /dev/null; then
        missing+=("jq (brew install jq)")
    fi
    
    if [ ${#missing[@]} -gt 0 ]; then
        echo -e "${RED}Missing dependencies:${NC}"
        for dep in "${missing[@]}"; do
            echo -e "  - ${dep}"
        done
        exit 1
    fi
}

print_banner
echo ""
echo -e "${GREEN}Configuration:${NC}"
echo "  Coordinators: ${NUM_COORDS}"
echo "  Volumes: ${NUM_VOLUMES}"
echo "  Replicas: ${REPLICAS}"
echo "  Virtual users: ${VUS}"
echo "  Duration: ${DURATION}"
echo "  Object size: $((OBJECT_SIZE / 1024 / 1024)) MB"
echo ""

check_deps
echo -e "${GREEN}[OK] All dependencies found${NC}"
echo ""

# Build
echo -e "${YELLOW}Building release binaries...${NC}"
cargo build --release --quiet
echo -e "${GREEN}[OK] Build complete${NC}"
echo ""

# Create temp directories
BENCH_DIR="./bench_temp_$$"
mkdir -p "${BENCH_DIR}"
trap "rm -rf ${BENCH_DIR}; pkill -P $$ 2>/dev/null || true" EXIT

# Start coordinators
echo -e "${YELLOW}Starting ${NUM_COORDS} coordinators...${NC}"
for i in $(seq 1 ${NUM_COORDS}); do
    COORD_HTTP=$((5000 + (i-1)*2))
    COORD_GRPC=$((5001 + (i-1)*2))
    
    # Build peers list (exclude self)
    PEERS=""
    for j in $(seq 1 ${NUM_COORDS}); do
        if [ $j -ne $i ]; then
            PEER_GRPC=$((5001 + (j-1)*2))
            if [ -z "$PEERS" ]; then
                PEERS="coord-$j:$PEER_GRPC"
