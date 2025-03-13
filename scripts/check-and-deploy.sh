#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
NAMESPACE="icn-system"
REMOTE_DIR="~/icn-deploy"

# Print header
echo "================================================================"
echo "ICN Network Simplified Deployment Script"
echo "================================================================"
echo "This script will connect to ${REMOTE_HOST} and deploy"
echo "the ICN network using the already transferred files."
echo ""
echo "When prompted, enter your SSH key passphrase."
echo "================================================================"

# Connect to the remote server and apply the files
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  set -e
  cd ${REMOTE_DIR}
  
  echo 'Checking disk space...'
  df -h /
  
  echo 'Checking node status...'
  sudo kubectl get nodes
  
  echo 'Checking if registry config is in place...'
  if [ ! -f /etc/rancher/k3s/registries.yaml ]; then
    echo 'Setting up HTTP registry access...'
    cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
EOF

    echo 'Restarting k3s to apply registry config...'
    sudo systemctl restart k3s
    sleep 30
  else
    echo 'Registry config already exists.'
  fi
  
  echo 'Applying ConfigMap...'
  sudo kubectl apply -f configmap.yaml
  
  echo 'Deploying primary nodes...'
  sudo kubectl apply -f coop1-primary-service.yaml
  sudo kubectl apply -f coop1-primary-deployment.yaml
  
  echo 'Checking initial deployment status...'
  sudo kubectl get pods -n ${NAMESPACE}
  
  echo 'Waiting for 30 seconds to see if image pulls successfully...'
  sleep 30
  
  echo 'Deployment status after 30 seconds:'
  sudo kubectl get pods -n ${NAMESPACE}
  
  # Check if pods are being created before proceeding
  if sudo kubectl get pods -n ${NAMESPACE} | grep -q Running; then
    echo 'Pods are running, proceeding with remaining resources...'
    sudo kubectl apply -f coop2-primary-service.yaml
    sudo kubectl apply -f coop2-primary-deployment.yaml
    sudo kubectl apply -f coop1-secondary-service.yaml
    sudo kubectl apply -f coop1-secondary-deployment.yaml
    sudo kubectl apply -f coop2-secondary-service.yaml
    sudo kubectl apply -f coop2-secondary-deployment.yaml
    
    echo 'Waiting for a minute to allow pods to start...'
    sleep 60
    
    echo 'Final deployment status:'
    sudo kubectl get pods -n ${NAMESPACE}
    
    echo ''
    echo 'Checking events (most recent first):'
    sudo kubectl get events --sort-by=.metadata.creationTimestamp -n ${NAMESPACE} | tail -15
    
    echo ''
    echo 'Pod details:'
    sudo kubectl describe pods -n ${NAMESPACE} | grep -A 10 'Events:'
  else
    echo 'Pods are not running. Please check the events:'
    sudo kubectl get events --sort-by=.metadata.creationTimestamp -n ${NAMESPACE} | tail -15
    echo ''
    echo 'Please fix the issues before deploying the remaining resources.'
  fi
"

echo "================================================================"
echo "Deployment attempt completed."
echo "To check status later, run: ./scripts/check-icn-status.sh"
echo "================================================================" 