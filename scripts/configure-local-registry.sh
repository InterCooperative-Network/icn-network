#!/bin/bash
set -e

echo "================================================================"
echo "Configuring Local Docker for Insecure Registry"
echo "================================================================"
echo "This script will configure the local Docker daemon to allow"
echo "insecure registry access to 10.10.100.102:30500."
echo "================================================================"

# Create Docker daemon configuration directory
echo "Creating Docker daemon configuration directory..."
sudo mkdir -p /etc/docker

# Create or update daemon.json
echo "Configuring Docker daemon for insecure registry..."
cat << EOF | sudo tee /etc/docker/daemon.json
{
  "insecure-registries": ["10.10.100.102:30500"]
}
EOF

# Restart Docker daemon
echo "Restarting Docker daemon..."
sudo systemctl restart docker

# Wait for Docker to restart
echo "Waiting for Docker to restart..."
sleep 5

# Test Docker
echo "Testing Docker..."
docker info | grep "Insecure Registries"

echo "================================================================"
echo "Local Docker configuration completed."
echo "Please run 'docker info' to verify the insecure registry is listed."
echo "================================================================" 