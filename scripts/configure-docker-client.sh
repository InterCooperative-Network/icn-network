#!/bin/bash
set -e

echo "================================================================"
echo "Configuring Docker Client"
echo "================================================================"
echo "This script will configure the Docker client to allow"
echo "insecure registry access to 10.10.100.102:30500."
echo "================================================================"

# Create Docker config directory
echo "Creating Docker config directory..."
mkdir -p ~/.docker

# Create or update config.json
echo "Configuring Docker client for insecure registry..."
cat << EOF > ~/.docker/config.json
{
  "auths": {},
  "insecure-registries": ["10.10.100.102:30500"]
}
EOF

echo "================================================================"
echo "Docker client configuration completed."
echo "================================================================"
echo "Now try building and pushing the image again:"
echo "1. docker build -t 10.10.100.102:30500/icn-test-node:latest -f Dockerfile.k8s ."
echo "2. docker push 10.10.100.102:30500/icn-test-node:latest"
echo "================================================================" 