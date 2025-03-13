#!/bin/bash

# This script creates the registry configuration for containerd
# It should be run on each node individually

echo "Creating registry configuration for containerd..."
sudo mkdir -p /etc/rancher/k3s/

cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
EOF

echo "Restarting k3s service..."
if systemctl is-active --quiet k3s; then
  sudo systemctl restart k3s
  echo "k3s service restarted."
elif systemctl is-active --quiet k3s-agent; then
  sudo systemctl restart k3s-agent
  echo "k3s-agent service restarted."
else
  echo "No k3s or k3s-agent service found."
fi

echo "Registry configuration completed!" 