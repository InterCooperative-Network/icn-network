#!/bin/bash

# ICN Testnet Startup Script

set -e

# Configuration
TESTNET_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CONFIG_DIR="$TESTNET_ROOT/config"
DATA_DIR="$TESTNET_ROOT/data"
LOG_DIR="$TESTNET_ROOT/logs"
KEYS_DIR="$TESTNET_ROOT/keys"

# Create necessary directories
mkdir -p "$DATA_DIR" "$LOG_DIR" "$KEYS_DIR"

# Function to start a node
start_node() {
    local node_type=$1
    local node_id=$2
    local port=$3
    local extra_args=$4

    echo "Starting $node_type node $node_id on port $port..."
    
    # Create node-specific directories
    local node_data_dir="$DATA_DIR/${node_type}_${node_id}"
    local node_keys_dir="$KEYS_DIR/${node_type}_${node_id}"
    mkdir -p "$node_data_dir" "$node_keys_dir"

    # Generate node configuration
    cat > "$node_data_dir/config.toml" << EOF
[node]
type = "$node_type"
id = "$node_id"
data_dir = "$node_data_dir"
keys_dir = "$node_keys_dir"

[network]
# Primary IPv6 listener
listen_addr = "/ip6/::/tcp/$port"
external_addr = "/ip6/::1/tcp/$port"

# IPv4 fallback listener
ipv4_fallback = true
ipv4_listen_addr = "/ip4/0.0.0.0/tcp/$port"
ipv4_external_addr = "/ip4/127.0.0.1/tcp/$port"

# Network parameters
enable_relay_client = true
enable_mdns = true
enable_kad_dht = true

[storage]
path = "$node_data_dir/db"

[metrics]
enabled = true
port = $((port + 1000))

[logging]
file = "$LOG_DIR/${node_type}_${node_id}.log"
level = "info"
EOF

    # Start the node
    cargo run --bin icn-node -- \
        --config "$node_data_dir/config.toml" \
        $extra_args \
        >> "$LOG_DIR/${node_type}_${node_id}.log" 2>&1 &

    echo $! > "$node_data_dir/node.pid"
    echo "Node started with PID $(cat "$node_data_dir/node.pid")"
}

# Function to wait for node availability
wait_for_node() {
    local port=$1
    local timeout=30
    local count=0

    echo "Waiting for node on port $port..."
    # Try IPv6 first, then fall back to IPv4
    while ! (nc -6 -z ::1 $port 2>/dev/null || nc -4 -z localhost $port 2>/dev/null) && [ $count -lt $timeout ]; do
        sleep 1
        ((count++))
    done

    if [ $count -eq $timeout ]; then
        echo "Timeout waiting for node on port $port"
        exit 1
    fi
}

# Start bootstrap nodes
for i in {0..2}; do
    port=$((9000 + i))
    start_node "bootstrap" $i $port "--bootstrap"
    wait_for_node $port
done

# Start relay nodes
for i in {0..1}; do
    port=$((9010 + i))
    start_node "relay" $i $port "--relay"
    wait_for_node $port
done

# Start regular nodes
for i in {0..4}; do
    port=$((9020 + i))
    start_node "regular" $i $port ""
    wait_for_node $port
done

echo "Testnet started successfully!"
echo "Use ./stop_testnet.sh to stop the network"

# Create a summary of running nodes
echo "Running nodes:"
echo "-------------"
echo "Bootstrap nodes:"
echo "- IPv6: [::1]:9000-9002"
echo "- IPv4 fallback: 127.0.0.1:9000-9002"
echo "Relay nodes:"
echo "- IPv6: [::1]:9010-9011"
echo "- IPv4 fallback: 127.0.0.1:9010-9011"
echo "Regular nodes:"
echo "- IPv6: [::1]:9020-9024"
echo "- IPv4 fallback: 127.0.0.1:9020-9024"
echo "Metrics available at:"
echo "- Bootstrap nodes: ports 10000-10002"
echo "- Relay nodes: ports 10010-10011"
echo "- Regular nodes: ports 10020-10024" 