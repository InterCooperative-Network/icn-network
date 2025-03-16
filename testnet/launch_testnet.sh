#!/bin/bash
# ICN IPv6 Overlay Network Testnet Launcher
# This script launches multiple testnet nodes with different configurations

set -e

# Configuration
BASE_PORT=9000
BINARY="cargo run --bin testnet_node --"
LOG_DIR="./logs"

# Create log directory if it doesn't exist
mkdir -p $LOG_DIR

# Kill previous testnet instances if they exist
echo "Stopping any previous testnet instances..."
pkill -f "testnet_node" || true
sleep 2

# Start bootstrap nodes (one per federation)
echo "Starting bootstrap nodes..."

# Federation A bootstrap
echo "Starting Federation A bootstrap node..."
$BINARY --node-id="federation-a-bootstrap" \
        --federation="federation-a" \
        --port=$((BASE_PORT)) \
        --forwarding-policy="ForwardKnown" \
        --log-level=info > $LOG_DIR/federation-a-bootstrap.log 2>&1 &
FEDERATION_A_BOOTSTRAP_PID=$!
sleep 2

# Get Federation A bootstrap address
FEDERATION_A_ADDR=$(grep -m 1 "Node initialized with overlay address:" $LOG_DIR/federation-a-bootstrap.log | sed 's/.*overlay address: //')
echo "Federation A bootstrap address: $FEDERATION_A_ADDR"

# Federation B bootstrap
echo "Starting Federation B bootstrap node..."
$BINARY --node-id="federation-b-bootstrap" \
        --federation="federation-b" \
        --port=$((BASE_PORT+1)) \
        --forwarding-policy="ForwardKnown" \
        --log-level=info > $LOG_DIR/federation-b-bootstrap.log 2>&1 &
FEDERATION_B_BOOTSTRAP_PID=$!
sleep 2

# Get Federation B bootstrap address
FEDERATION_B_ADDR=$(grep -m 1 "Node initialized with overlay address:" $LOG_DIR/federation-b-bootstrap.log | sed 's/.*overlay address: //')
echo "Federation B bootstrap address: $FEDERATION_B_ADDR"

# Federation C bootstrap
echo "Starting Federation C bootstrap node..."
$BINARY --node-id="federation-c-bootstrap" \
        --federation="federation-c" \
        --port=$((BASE_PORT+2)) \
        --forwarding-policy="ForwardKnown" \
        --log-level=info > $LOG_DIR/federation-c-bootstrap.log 2>&1 &
FEDERATION_C_BOOTSTRAP_PID=$!
sleep 2

# Get Federation C bootstrap address
FEDERATION_C_ADDR=$(grep -m 1 "Node initialized with overlay address:" $LOG_DIR/federation-c-bootstrap.log | sed 's/.*overlay address: //')
echo "Federation C bootstrap address: $FEDERATION_C_ADDR"

# Independent node (no federation)
echo "Starting independent node (no federation)..."
$BINARY --node-id="independent-node" \
        --federation="" \
        --port=$((BASE_PORT+3)) \
        --forwarding-policy="ForwardKnown" \
        --log-level=info > $LOG_DIR/independent-node.log 2>&1 &
INDEPENDENT_NODE_PID=$!
sleep 2

# Get independent node address
INDEPENDENT_ADDR=$(grep -m 1 "Node initialized with overlay address:" $LOG_DIR/independent-node.log | sed 's/.*overlay address: //')
echo "Independent node address: $INDEPENDENT_ADDR"

# Start member nodes for Federation A
echo "Starting member nodes for Federation A..."
for i in {1..3}; do
    echo "Starting Federation A node $i..."
    $BINARY --node-id="federation-a-node-$i" \
            --federation="federation-a" \
            --port=$((BASE_PORT+10+i)) \
            --bootstrap-peers="$FEDERATION_A_ADDR" \
            --forwarding-policy="ForwardKnown" \
            --log-level=info > $LOG_DIR/federation-a-node-$i.log 2>&1 &
    sleep 1
done

# Start member nodes for Federation B
echo "Starting member nodes for Federation B..."
for i in {1..3}; do
    echo "Starting Federation B node $i..."
    $BINARY --node-id="federation-b-node-$i" \
            --federation="federation-b" \
            --port=$((BASE_PORT+20+i)) \
            --bootstrap-peers="$FEDERATION_B_ADDR" \
            --forwarding-policy="ForwardKnown" \
            --log-level=info > $LOG_DIR/federation-b-node-$i.log 2>&1 &
    sleep 1
done

# Start member nodes for Federation C
echo "Starting member nodes for Federation C..."
for i in {1..2}; do
    echo "Starting Federation C node $i..."
    $BINARY --node-id="federation-c-node-$i" \
            --federation="federation-c" \
            --port=$((BASE_PORT+30+i)) \
            --bootstrap-peers="$FEDERATION_C_ADDR" \
            --forwarding-policy="ForwardKnown" \
            --log-level=info > $LOG_DIR/federation-c-node-$i.log 2>&1 &
    sleep 1
done

# Start nodes that connect to multiple federations
echo "Starting cross-federation node A-B..."
$BINARY --node-id="cross-federation-ab" \
        --federation="federation-a" \
        --port=$((BASE_PORT+40)) \
        --bootstrap-peers="$FEDERATION_A_ADDR,$FEDERATION_B_ADDR" \
        --forwarding-policy="ForwardAll" \
        --log-level=info > $LOG_DIR/cross-federation-ab.log 2>&1 &
sleep 1

echo "Starting cross-federation node B-C..."
$BINARY --node-id="cross-federation-bc" \
        --federation="federation-b" \
        --port=$((BASE_PORT+41)) \
        --bootstrap-peers="$FEDERATION_B_ADDR,$FEDERATION_C_ADDR" \
        --forwarding-policy="ForwardAll" \
        --log-level=info > $LOG_DIR/cross-federation-bc.log 2>&1 &
sleep 1

echo "Starting cross-federation node A-C..."
$BINARY --node-id="cross-federation-ac" \
        --federation="federation-c" \
        --port=$((BASE_PORT+42)) \
        --bootstrap-peers="$FEDERATION_A_ADDR,$FEDERATION_C_ADDR" \
        --forwarding-policy="ForwardAll" \
        --log-level=info > $LOG_DIR/cross-federation-ac.log 2>&1 &
sleep 1

# Print testnet status
echo ""
echo "================ ICN IPv6 Overlay Network Testnet ================"
echo "Total nodes: 16"
echo " - Federation A: 1 bootstrap + 3 members + 2 cross-federation"
echo " - Federation B: 1 bootstrap + 3 members + 2 cross-federation"
echo " - Federation C: 1 bootstrap + 2 members + 2 cross-federation"
echo " - Independent: 1 node"
echo ""
echo "Log files are in $LOG_DIR"
echo "=================================================================="
echo ""
echo "The testnet is now running. Press Ctrl+C to shut down."
echo ""

# Trap for clean shutdown
trap 'echo "Shutting down testnet..."; pkill -f "testnet_node"; exit 0' INT TERM

# Wait for Ctrl+C
while true; do
    sleep 1
done 