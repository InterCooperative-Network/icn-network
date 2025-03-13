#!/bin/bash
set -e

echo "================================================================"
echo "Configuring Registry and Pushing Image"
echo "================================================================"
echo "This script will configure the registry for HTTP access and"
echo "push the Docker image."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Configure local Docker for insecure registry first
echo "Configuring local Docker for insecure registry..."
sudo mkdir -p /etc/docker
cat << EOF | sudo tee /etc/docker/daemon.json
{
  "insecure-registries" : ["10.10.100.102:30500"]
}
EOF

echo "Restarting Docker daemon..."
sudo systemctl restart docker

# Wait for Docker to restart
echo "Waiting for Docker to restart..."
sleep 5

# Configure registry on all remote nodes
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Configuring containerd for insecure registry...'
sudo mkdir -p /etc/rancher/k3s

# Create registries.yaml with both mirror and host configuration
cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
configs:
  '10.10.100.102:30500':
    tls:
      insecure_skip_verify: true
EOF

# Also configure Docker on the remote host
echo 'Configuring Docker on remote host...'
sudo mkdir -p /etc/docker
cat << 'EOF' | sudo tee /etc/docker/daemon.json
{
  \"insecure-registries\" : [\"10.10.100.102:30500\"]
}
EOF

echo 'Restarting Docker on remote host...'
sudo systemctl restart docker || echo 'Docker restart failed (might not be installed)'

echo 'Restarting k3s to apply registry configuration...'
sudo systemctl restart k3s

echo 'Waiting for services to restart...'
sleep 10

echo 'Testing registry access...'
curl -v http://10.10.100.102:30500/v2/
"

# Build and push the image
echo "Building Docker image..."
docker build -t icn-test-node:latest -f Dockerfile.k8s .

echo "Tagging image for registry..."
docker tag icn-test-node:latest 10.10.100.102:30500/icn-test-node:latest

echo "Pushing image to registry..."
docker push 10.10.100.102:30500/icn-test-node:latest

echo "================================================================"
echo "Registry configured and image pushed successfully."
echo "================================================================"

# Verify the image is in the registry
echo "Verifying image in registry..."
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
curl -s http://10.10.100.102:30500/v2/_catalog
" 