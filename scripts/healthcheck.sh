#!/bin/bash
set -e

# Check if the process is running
if ! pgrep -f "icn-node" > /dev/null; then
    echo "ICN node process is not running"
    exit 1
fi

# Check if the node is listening on its port
PORT=$(echo $ICN_LISTEN_ADDR | cut -d':' -f2)
if ! netstat -tln | grep -q ":$PORT\\b"; then
    echo "ICN node is not listening on port $PORT"
    exit 1
fi

# Check log file for recent activity (within last 30 seconds)
LOG_FILE="$ICN_LOG_DIR/$ICN_NODE_ID.log"
if [ -f "$LOG_FILE" ]; then
    if ! find "$LOG_FILE" -mmin -0.5 > /dev/null; then
        echo "No recent log activity"
        exit 1
    fi
else
    echo "Log file not found"
    exit 1
fi

# All checks passed
echo "Health check passed"
exit 0 