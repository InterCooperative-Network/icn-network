#!/bin/bash

# ICN Testnet Shutdown Script

set -e

# Configuration
TESTNET_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DATA_DIR="$TESTNET_ROOT/data"

# Function to stop a node
stop_node() {
    local node_type=$1
    local node_id=$2
    local node_data_dir="$DATA_DIR/${node_type}_${node_id}"
    local pid_file="$node_data_dir/node.pid"

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        echo "Stopping $node_type node $node_id (PID: $pid)..."
        
        # Send SIGTERM to allow graceful shutdown
        if kill -15 "$pid" 2>/dev/null; then
            # Wait for up to 10 seconds for graceful shutdown
            local count=0
            while kill -0 "$pid" 2>/dev/null && [ $count -lt 10 ]; do
                sleep 1
                ((count++))
            done
            
            # If still running, force kill
            if kill -0 "$pid" 2>/dev/null; then
                echo "Force killing $node_type node $node_id..."
                kill -9 "$pid" 2>/dev/null || true
            fi
        fi
        
        rm -f "$pid_file"
    fi
}

# Stop all nodes in reverse order
echo "Stopping testnet nodes..."

# Stop regular nodes
for i in {4..0}; do
    stop_node "regular" $i
done

# Stop relay nodes
for i in {1..0}; do
    stop_node "relay" $i
done

# Stop bootstrap nodes
for i in {2..0}; do
    stop_node "bootstrap" $i
done

echo "All nodes stopped successfully!"

# Optionally clean up data
read -p "Clean up testnet data? [y/N] " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    echo "Cleaning up testnet data..."
    rm -rf "$DATA_DIR"/*
    echo "Cleanup complete!"
fi 