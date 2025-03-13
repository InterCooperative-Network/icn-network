#!/bin/bash
set -e

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
LOCAL_IMAGE="icn-network:latest"
NAMESPACE="icn-network-$(date +%Y%m%d%H%M%S)"
TMP_DIR="/tmp/icn-k8s-${NAMESPACE}"

# Print header
echo "================================================================"
echo "ICN Network Deployment to Remote Kubernetes Cluster"
echo "================================================================"
echo "This script will deploy ICN Network to ${REMOTE_HOST}"
echo "You will be prompted for the SSH key passphrase several times"
echo "Namespace: ${NAMESPACE}"
echo ""

# 1. Build the Docker image locally
echo "[1/7] Building Docker image locally..."
docker build -t ${LOCAL_IMAGE} -f Dockerfile.k8s .

# 2. Save the Docker image to a tar file
echo "[2/7] Saving Docker image to tar file..."
docker save -o icn-network.tar ${LOCAL_IMAGE}

# 3. Create temporary directory on remote host
echo "[3/7] Creating temporary directory on remote host..."
echo "When prompted, enter your SSH key passphrase."
ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "mkdir -p ${TMP_DIR}"

# 4. Copy the Docker image to the remote host
echo "[4/7] Copying Docker image to remote host..."
echo "When prompted, enter your SSH key passphrase."
scp -i ${SSH_KEY} icn-network.tar matt@${REMOTE_HOST}:${TMP_DIR}/

# 5. Update all Kubernetes YAML files to use the correct namespace
echo "[5/7] Updating Kubernetes YAML files..."
for file in kubernetes/*.yaml; do
  sed -i "s|\${NAMESPACE}|${NAMESPACE}|g" "$file"
done

# 6. Copy all Kubernetes YAML files to the remote host
echo "[6/7] Copying Kubernetes YAML files to remote host..."
echo "When prompted, enter your SSH key passphrase."
scp -i ${SSH_KEY} kubernetes/*.yaml matt@${REMOTE_HOST}:${TMP_DIR}/

# 7. SSH into the remote host and perform deployment
echo "[7/7] Deploying to remote Kubernetes cluster..."
echo "When prompted, enter your SSH key passphrase."
ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "
  set -e
  
  # Setup kubeconfig
  if [ -f ~/.kube/config ]; then
    export KUBECONFIG=~/.kube/config
    echo 'Using ~/.kube/config'
  else
    echo 'No kubeconfig found in ~/.kube/config. Trying default location...'
  fi
  
  # Load the Docker image
  echo 'Loading Docker image on remote host...'
  docker load -i ${TMP_DIR}/icn-network.tar
  docker images | grep icn-network
  
  # Create namespace
  echo 'Creating namespace...'
  kubectl create namespace ${NAMESPACE}
  
  # Apply ConfigMap
  echo 'Applying ConfigMap...'
  kubectl apply -f ${TMP_DIR}/configmap.yaml
  
  # Apply persistent volume claims
  echo 'Creating persistent volume claims...'
  kubectl apply -f ${TMP_DIR}/persistent-volume-claims.yaml
  
  # Deploy primary nodes
  echo 'Deploying primary nodes...'
  kubectl apply -f ${TMP_DIR}/coop1-primary-deployment.yaml
  kubectl apply -f ${TMP_DIR}/coop1-primary-service.yaml
  kubectl apply -f ${TMP_DIR}/coop2-primary-deployment.yaml
  kubectl apply -f ${TMP_DIR}/coop2-primary-service.yaml
  
  # Wait for primary nodes to be ready (with timeout)
  echo 'Waiting for primary nodes to be ready...'
  timeout 300s kubectl rollout status deployment/coop1-primary -n ${NAMESPACE} || echo 'Timeout waiting for coop1-primary'
  timeout 300s kubectl rollout status deployment/coop2-primary -n ${NAMESPACE} || echo 'Timeout waiting for coop2-primary'
  
  # Deploy secondary nodes
  echo 'Deploying secondary nodes...'
  kubectl apply -f ${TMP_DIR}/coop1-secondary-deployment.yaml
  kubectl apply -f ${TMP_DIR}/coop1-secondary-service.yaml
  kubectl apply -f ${TMP_DIR}/coop2-secondary-deployment.yaml
  kubectl apply -f ${TMP_DIR}/coop2-secondary-service.yaml
  
  # Wait for secondary nodes to be ready (with timeout)
  echo 'Waiting for secondary nodes to be ready...'
  timeout 300s kubectl rollout status deployment/coop1-secondary -n ${NAMESPACE} || echo 'Timeout waiting for coop1-secondary'
  timeout 300s kubectl rollout status deployment/coop2-secondary -n ${NAMESPACE} || echo 'Timeout waiting for coop2-secondary'
  
  # Show deployment status
  echo 'Deployment status:'
  kubectl get pods -n ${NAMESPACE} -o wide
  
  # Don't clean up automatically to allow for debugging
  echo 'Temporary files are in ${TMP_DIR} for debugging purposes'
"

# Clean up local tar file
echo "Cleaning up local files..."
rm -f icn-network.tar

echo "================================================================"
echo "Deployment completed."
echo "Namespace: ${NAMESPACE}"
echo ""
echo "To check status, run:"
echo "ssh -i ${SSH_KEY} matt@${REMOTE_HOST} \"kubectl get pods -n ${NAMESPACE}\""
echo ""
echo "To clean up the temporary directory on the remote server, run:"
echo "ssh -i ${SSH_KEY} matt@${REMOTE_HOST} \"rm -rf ${TMP_DIR}\""
echo "================================================================" 