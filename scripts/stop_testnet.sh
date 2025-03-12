#!/bin/bash
# ICN Testnet Stop Script
# This script stops the running ICN testnet

set -e

echo "Stopping ICN Testnet..."

# Check if the PID file exists
if [ ! -f "./testnet/testnet.pid" ]; then
    echo "Error: Testnet PID file not found. Is the testnet running?"
    exit 1
fi

# Read the PID
TESTNET_PID=$(cat ./testnet/testnet.pid)

# Check if the process is running
if ! ps -p $TESTNET_PID > /dev/null; then
    echo "Warning: Testnet process (PID $TESTNET_PID) is not running."
    echo "Removing PID file..."
    rm ./testnet/testnet.pid
    exit 0
fi

# Send SIGTERM to gracefully stop the testnet
echo "Stopping testnet process (PID $TESTNET_PID)..."
kill -TERM $TESTNET_PID

# Wait for the process to terminate
echo "Waiting for testnet to terminate..."
for i in {1..10}; do
    if ! ps -p $TESTNET_PID > /dev/null; then
        echo "Testnet stopped successfully."
        rm ./testnet/testnet.pid
        exit 0
    fi
    sleep 1
done

# If the process is still running, force kill it
echo "Warning: Testnet did not terminate gracefully. Forcing termination..."
kill -9 $TESTNET_PID
rm ./testnet/testnet.pid

echo "Testnet stopped." 