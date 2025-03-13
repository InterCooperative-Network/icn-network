#!/bin/bash
set -e

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
NAMESPACE="icn-system"
REMOTE_DIR="~/icn-deploy"
IMAGE="10.10.100.102:30500/icn-test-node:latest"

# Print header
echo "================================================================"
echo "ICN Network Revised Deployment Script"
echo "================================================================"
echo "This script will:"
echo "1. Update YAML files to use namespace: ${NAMESPACE}"
echo "2. Update deployment files to use image: ${IMAGE}"
echo "3. Transfer them to the remote server: ${REMOTE_HOST}:${REMOTE_DIR}"
echo "4. Connect to the remote server to apply the files"
echo ""
echo "You will be prompted for your SSH key passphrase multiple times."
echo "================================================================"

# Step 1: Update all YAML files 
echo "[1/3] Updating Kubernetes YAML files"

# Create a backup directory
BACKUP_DIR="kubernetes_backup_$(date +%Y%m%d%H%M%S)"
mkdir -p ${BACKUP_DIR}
cp kubernetes/*.yaml ${BACKUP_DIR}/
echo "Backed up original YAML files to ${BACKUP_DIR}/"

# Update all deployment YAML files to use the correct image and namespace
for file in kubernetes/*deployment.yaml; do
  # Skip namespace.yaml
  if [ $(basename "$file") == "namespace.yaml" ]; then
    continue
  fi
  
  # Update namespace references
  sed -i -E "s/namespace: icn-network-[0-9]+/namespace: ${NAMESPACE}/g" "$file"
  
  # Update image references - specifically match the image: line in deployment files
  sed -i "s|image: icn-network:latest|image: ${IMAGE}|g" "$file"
  sed -i "s|imagePullPolicy: Never|imagePullPolicy: IfNotPresent|g" "$file"
  
  echo "Updated $file"
done

# Also update any other YAML files to use the correct namespace
for file in kubernetes/*.yaml; do
  if [[ "$file" != *"deployment.yaml" ]] && [[ $(basename "$file") != "namespace.yaml" ]]; then
    # Update namespace references
    sed -i -E "s/namespace: icn-network-[0-9]+/namespace: ${NAMESPACE}/g" "$file"
    
    echo "Updated $file"
  fi
done

# Step 2: Transfer files to the remote server
echo "[2/3] Transferring Kubernetes YAML files to remote server"
echo "When prompted, enter your SSH key passphrase."

# Ensure the remote directory exists
ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "mkdir -p ${REMOTE_DIR}"

# Copy all kubernetes YAML files (excluding namespace.yaml)
for file in kubernetes/*.yaml; do
  if [ $(basename "$file") != "namespace.yaml" ]; then
    scp -i ${SSH_KEY} "$file" matt@${REMOTE_HOST}:${REMOTE_DIR}/
  fi
done

# Step 3: Connect to the remote server and apply the files
echo "[3/3] Connecting to remote server to apply Kubernetes manifests"
echo "When prompted, enter your SSH key passphrase."

ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  set -e
  cd ${REMOTE_DIR}
  
  echo 'Checking disk space before deployment...'
  df -h /
  
  echo 'Applying ConfigMap...'
  sudo kubectl apply -f configmap.yaml
  
  echo 'Creating persistent volume claims...'
  # Allow the persistent-volume-claims command to fail, as they might already exist
  sudo kubectl apply -f persistent-volume-claims.yaml || echo 'Warning: Some PVCs may already exist'
  
  echo 'Deploying primary nodes...'
  sudo kubectl apply -f coop1-primary-deployment.yaml
  sudo kubectl apply -f coop1-primary-service.yaml
  sudo kubectl apply -f coop2-primary-deployment.yaml
  sudo kubectl apply -f coop2-primary-service.yaml
  
  echo 'Deploying secondary nodes...'
  sudo kubectl apply -f coop1-secondary-deployment.yaml
  sudo kubectl apply -f coop1-secondary-service.yaml
  sudo kubectl apply -f coop2-secondary-deployment.yaml
  sudo kubectl apply -f coop2-secondary-service.yaml
  
  echo 'Deployment status:'
  sudo kubectl get pods -n ${NAMESPACE}
  
  echo 'Waiting for a minute to allow pods to start...'
  sleep 30
  
  echo 'After 30 seconds - deployment status:'
  sudo kubectl get pods -n ${NAMESPACE}
  
  echo ''
  echo 'Checking events (most recent first):'
  sudo kubectl get events --sort-by=.metadata.creationTimestamp -n ${NAMESPACE} | tail -10
"

echo "================================================================"
echo "Revised deployment completed."
echo "To check status later, run: ./scripts/check-icn-status.sh"
echo "================================================================" 