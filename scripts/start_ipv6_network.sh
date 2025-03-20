#!/bin/bash
# Start ICN IPv6 Network Nodes

# Kill any previous instances
pkill -f "icn-node" || true

# Check IPv6 support
if ! ip -6 addr show | grep -q "scope global"; then
    echo "Using local IPv6 addresses for testing (::1)"
else
    echo "Using global IPv6 addresses"
fi

# Start each node in a separate terminal - trying multiple terminal emulators
TERMINAL=""
if command -v gnome-terminal &> /dev/null; then
    TERMINAL="gnome-terminal --"
elif command -v xterm &> /dev/null; then
    TERMINAL="xterm -e"
elif command -v konsole &> /dev/null; then
    TERMINAL="konsole -e"
elif command -v terminal &> /dev/null; then
    TERMINAL="terminal -e"
else
    echo "No supported terminal emulator found. Starting nodes in background."
    TERMINAL=""
fi

# Start each node
if [ -n "$TERMINAL" ]; then
    $TERMINAL bash -c "cargo run --bin icn-node -- --config ./config/nodes/node-001.yaml; exec bash" &
else
    # Start in background if no terminal available
    cargo run --bin icn-node -- --config ./config/nodes/node-001.yaml > ./data/log/icn/node-001.log 2>&1 &
fi
sleep 2
if [ -n "$TERMINAL" ]; then
    $TERMINAL bash -c "cargo run --bin icn-node -- --config ./config/nodes/node-002.yaml; exec bash" &
else
    # Start in background if no terminal available
    cargo run --bin icn-node -- --config ./config/nodes/node-002.yaml > ./data/log/icn/node-002.log 2>&1 &
fi
sleep 2
if [ -n "$TERMINAL" ]; then
    $TERMINAL bash -c "cargo run --bin icn-node -- --config ./config/nodes/node-003.yaml; exec bash" &
else
    # Start in background if no terminal available
    cargo run --bin icn-node -- --config ./config/nodes/node-003.yaml > ./data/log/icn/node-003.log 2>&1 &
fi
sleep 2
