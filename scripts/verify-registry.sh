#!/bin/bash
set -e

echo "================================================================"
echo "Verifying Registry Configuration"
echo "================================================================"
echo "This script will connect to 10.10.100.102 and verify that the"
echo "registry is properly configured for HTTP access."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

# Check for registry configuration in containerd
echo 'Checking containerd registry configuration...'
if sudo grep -q '10.10.100.102:30500' /var/lib/rancher/k3s/agent/etc/containerd/config.toml; then
  echo 'Registry configuration found in containerd config'
  sudo grep -A 10 'registry.mirrors' /var/lib/rancher/k3s/agent/etc/containerd/config.toml
else
  echo 'WARNING: Registry configuration not found in containerd config'
fi

# Check if the registry can be reached
echo ''
echo 'Testing registry connectivity...'
curl -v http://10.10.100.102:30500/v2/ || echo 'Failed to connect to registry'

# Check available images in the registry
echo ''
echo 'Checking available images in registry...'
curl -s http://10.10.100.102:30500/v2/_catalog || echo 'Failed to list registry images'

# Check if we can pull the image
echo ''
echo 'Testing image pull...'
sudo crictl pull --insecure-registry 10.10.100.102:30500/icn-test-node:latest || echo 'Failed to pull image'
"

echo "================================================================"
echo "Registry verification completed."
echo "================================================================" 