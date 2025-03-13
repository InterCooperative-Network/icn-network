#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

# Print header
echo "================================================================"
echo "Configure K3s Registry for HTTP Access"
echo "================================================================"
echo "This script will connect to ${REMOTE_HOST} and configure K3s"
echo "to use HTTP for the local registry."
echo ""
echo "When prompted, enter your SSH key passphrase."
echo "================================================================"

# Connect to the remote server and configure the registry
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  echo 'Setting up HTTP registry access...'
  cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
EOF

  echo 'Restarting k3s to apply registry config...'
  sudo systemctl restart k3s
  
  echo 'Waiting for 30 seconds for k3s to restart...'
  sleep 30
  
  echo 'Checking if k3s is running...'
  sudo systemctl status k3s | grep Active
  
  echo 'Checking node status...'
  sudo kubectl get nodes
"

echo "================================================================"
echo "Registry configuration completed."
echo "You can now continue with the deployment using:"
echo "./scripts/final-deployment.sh"
echo "================================================================" 