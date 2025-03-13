#!/bin/bash

echo "================================================================"
echo "Configuring containerd for HTTP registry"
echo "================================================================"
echo "This script will configure containerd to use HTTP for the registry"
echo "at 10.10.100.102:30500."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Connect to the remote server and configure containerd
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Creating registry configuration for containerd...'
sudo mkdir -p /etc/rancher/k3s/

cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
EOF

echo 'Restarting k3s to apply registry configuration...'
sudo systemctl restart k3s
sleep 10

echo 'Verifying registry configuration...'
if curl -s http://10.10.100.102:30500/v2/ &>/dev/null; then
  echo 'Registry is accessible via HTTP.'
else
  echo 'WARNING: Registry not accessible via HTTP. Please check the registry manually.'
fi

echo 'Checking if nodes are ready...'
sudo kubectl get nodes
"

echo "================================================================"
echo "containerd configuration completed!"
echo "Now you can run the deployment script again."
echo "================================================================" 