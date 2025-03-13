#!/bin/bash
set -e

echo "================================================================"
echo "Configuring Docker in WSL2 for Insecure Registry"
echo "================================================================"
echo "This script will configure Docker in WSL2 to allow"
echo "insecure registry access to 10.10.100.102:30500."
echo "================================================================"

# Create Docker config directory in home
echo "Creating Docker config directory..."
mkdir -p ~/.docker

# Create or update config.json
echo "Configuring Docker client for insecure registry..."
cat << EOF > ~/.docker/config.json
{
  "insecure-registries": ["10.10.100.102:30500"]
}
EOF

# Create daemon configuration directory
echo "Creating Docker daemon configuration directory..."
sudo mkdir -p /etc/docker

# Create or update daemon.json
echo "Configuring Docker daemon for insecure registry..."
cat << EOF | sudo tee /etc/docker/daemon.json
{
  "insecure-registries": ["10.10.100.102:30500"]
}
EOF

# Update Docker service configuration
echo "Updating Docker service configuration..."
sudo mkdir -p /etc/systemd/system/docker.service.d
cat << EOF | sudo tee /etc/systemd/system/docker.service.d/override.conf
[Service]
ExecStart=
ExecStart=/usr/bin/dockerd --insecure-registry=10.10.100.102:30500
EOF

# Reload systemd and restart Docker
echo "Reloading systemd configuration..."
sudo systemctl daemon-reload

echo "Restarting Docker daemon..."
sudo systemctl restart docker

# Wait for Docker to restart
echo "Waiting for Docker to restart..."
sleep 5

# Test Docker configuration
echo "Testing Docker configuration..."
docker info | grep -A1 "Insecure Registries"

echo "================================================================"
echo "Docker configuration completed."
echo "You may need to run the following commands if the push still fails:"
echo "1. docker logout 10.10.100.102:30500"
echo "2. docker system prune -f"
echo "Then try building and pushing the image again."
echo "================================================================" 