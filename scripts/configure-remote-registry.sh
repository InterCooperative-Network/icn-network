#!/bin/bash
set -e

echo "================================================================"
echo "Configuring Remote Registry"
echo "================================================================"
echo "This script will configure the registry on the remote Kubernetes"
echo "cluster at 10.10.100.102."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

# Configure containerd for insecure registry
echo 'Configuring containerd for insecure registry...'
sudo mkdir -p /etc/rancher/k3s

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

# Restart k3s to apply changes
echo 'Restarting k3s to apply registry configuration...'
sudo systemctl restart k3s

echo 'Waiting for k3s to restart...'
sleep 10

# Test registry access
echo 'Testing registry access...'
curl -v http://10.10.100.102:30500/v2/

# Show current images in registry
echo 'Current images in registry:'
curl -s http://10.10.100.102:30500/v2/_catalog
"

echo "================================================================"
echo "Remote registry configuration completed."
echo "================================================================"
echo "Now you can build and push the image:"
echo "1. docker build -t 10.10.100.102:30500/icn-test-node:latest -f Dockerfile.k8s ."
echo "2. docker push 10.10.100.102:30500/icn-test-node:latest"
echo "================================================================" 