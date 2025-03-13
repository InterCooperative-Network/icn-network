#!/bin/bash
set -e

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
LOCAL_IMAGE="icn-network:latest"
NAMESPACE="icn-network-$(date +%Y%m%d%H%M%S)"
TMP_DIR="/tmp/icn-k8s"

# Read SSH passphrase
read -sp "Enter SSH key passphrase: " PASSPHRASE
echo

# Function to run SSH commands with passphrase
run_ssh() {
  local cmd="$1"
  expect <<EOD
  spawn ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "$cmd"
  expect "Enter passphrase for key '${SSH_KEY}':"
  send "$PASSPHRASE\r"
  expect eof
EOD
}

# Function to copy files with passphrase
scp_file() {
  local src="$1"
  local dest="$2"
  expect <<EOD
  spawn scp -i ${SSH_KEY} "$src" matt@${REMOTE_HOST}:"$dest"
  expect "Enter passphrase for key '${SSH_KEY}':"
  send "$PASSPHRASE\r"
  expect eof
EOD
}

# 1. Build the Docker image locally
echo "Building Docker image locally..."
docker build -t ${LOCAL_IMAGE} -f Dockerfile.k8s .

# 2. Save the Docker image to a tar file
echo "Saving Docker image to tar file..."
docker save -o icn-network.tar ${LOCAL_IMAGE}

# 3. Create temporary directory on remote host
echo "Creating temporary directory on remote host..."
run_ssh "mkdir -p ${TMP_DIR}"

# 4. Copy the Docker image to the remote host
echo "Copying Docker image to remote host..."
scp_file "icn-network.tar" "${TMP_DIR}/"

# 5. Update all Kubernetes YAML files to use the correct namespace
echo "Updating Kubernetes YAML files..."
for file in kubernetes/*.yaml; do
  sed -i "s|\${NAMESPACE}|${NAMESPACE}|g" "$file"
done

# 6. Copy all Kubernetes YAML files to the remote host
echo "Copying Kubernetes YAML files to remote host..."
for file in kubernetes/*.yaml; do
  scp_file "$file" "${TMP_DIR}/$(basename $file)"
done

# 7. Check for kubconfig location on remote host and deploy
echo "Checking for kubeconfig and deploying to remote Kubernetes cluster..."
run_ssh "
  set -e
  
  # Check for kubeconfig locations
  if [ -f ~/.kube/config ]; then
    export KUBECONFIG=~/.kube/config
  elif [ -f /etc/rancher/k3s/k3s.yaml ]; then
    # For this case, we might need sudo
    if [ -r /etc/rancher/k3s/k3s.yaml ]; then
      export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
    else
      echo 'Cannot read k3s.yaml, trying with sudo'
      # Create a temporary copy with permissions
      sudo cp /etc/rancher/k3s/k3s.yaml ${TMP_DIR}/kubeconfig.yaml
      sudo chmod 644 ${TMP_DIR}/kubeconfig.yaml
      export KUBECONFIG=${TMP_DIR}/kubeconfig.yaml
    fi
  else
    echo 'Unable to find kubeconfig. Please specify path:'
    read -p 'Kubeconfig path: ' entered_kubeconfig
    if [ -f \"\$entered_kubeconfig\" ]; then
      export KUBECONFIG=\"\$entered_kubeconfig\"
    else
      echo 'Invalid kubeconfig path. Exiting.'
      exit 1
    fi
  fi
  
  # Verify kubectl works
  echo 'Testing kubectl access:'
  kubectl get nodes || { echo 'Failed to access Kubernetes. Check permissions.'; exit 1; }
  
  # Load the Docker image
  echo 'Loading Docker image on remote host...'
  docker load -i ${TMP_DIR}/icn-network.tar
  
  # Create namespace
  echo 'Creating namespace...'
  kubectl create namespace ${NAMESPACE} || true
  
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
  
  # Wait for primary nodes to be ready
  echo 'Waiting for primary nodes to be ready...'
  kubectl rollout status deployment/coop1-primary -n ${NAMESPACE}
  kubectl rollout status deployment/coop2-primary -n ${NAMESPACE}
  
  # Deploy secondary nodes
  echo 'Deploying secondary nodes...'
  kubectl apply -f ${TMP_DIR}/coop1-secondary-deployment.yaml
  kubectl apply -f ${TMP_DIR}/coop1-secondary-service.yaml
  kubectl apply -f ${TMP_DIR}/coop2-secondary-deployment.yaml
  kubectl apply -f ${TMP_DIR}/coop2-secondary-service.yaml
  
  # Wait for secondary nodes to be ready
  echo 'Waiting for secondary nodes to be ready...'
  kubectl rollout status deployment/coop1-secondary -n ${NAMESPACE}
  kubectl rollout status deployment/coop2-secondary -n ${NAMESPACE}
  
  # Clean up
  echo 'Cleaning up...'
  rm -rf ${TMP_DIR}
  
  # Show deployment status
  echo 'Deployment status:'
  kubectl get pods -n ${NAMESPACE}
"

# Clean up local tar file
echo "Cleaning up local files..."
rm -f icn-network.tar

echo "Deployment completed."
echo "Namespace: ${NAMESPACE}"
echo "To check status, use the same passphrase and run:"
echo "ssh -i ${SSH_KEY} matt@${REMOTE_HOST} \"export KUBECONFIG=~/.kube/config; kubectl get pods -n ${NAMESPACE}\"" 