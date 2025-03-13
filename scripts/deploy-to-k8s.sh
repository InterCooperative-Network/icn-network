#!/bin/bash
set -e

# Default values
IMAGE_REGISTRY=${IMAGE_REGISTRY:-localhost:5000}
IMAGE_NAME=${IMAGE_NAME:-icn-network}
IMAGE_TAG=${IMAGE_TAG:-latest}
TIMESTAMP=$(date +%Y%m%d%H%M%S)
NAMESPACE=${NAMESPACE:-icn-network-${TIMESTAMP}}
K8S_CONTEXT=""
REMOTE_CLUSTER=false
REMOTE_MASTER="10.10.100.102"
REMOTE_SSH_KEY="/home/matt/.ssh/id_rsa_new"

# Parse command line arguments
while [[ $# -gt 0 ]]; do
  case $1 in
    --registry)
      IMAGE_REGISTRY="$2"
      shift 2
      ;;
    --tag)
      IMAGE_TAG="$2"
      shift 2
      ;;
    --namespace)
      NAMESPACE="$2"
      shift 2
      ;;
    --build)
      BUILD_IMAGE=true
      shift
      ;;
    --local)
      USE_LOCAL_IMAGE=true
      shift
      ;;
    --minikube)
      USE_MINIKUBE=true
      shift
      ;;
    --remote)
      REMOTE_CLUSTER=true
      shift
      ;;
    --remote-master)
      REMOTE_MASTER="$2"
      shift 2
      ;;
    --remote-ssh-key)
      REMOTE_SSH_KEY="$2"
      shift 2
      ;;
    --context)
      K8S_CONTEXT="$2"
      shift 2
      ;;
    --clean)
      CLEAN_FIRST=true
      shift
      ;;
    *)
      echo "Unknown option: $1"
      exit 1
      ;;
  esac
done

# Function to run kubectl commands on remote cluster
run_remote_kubectl() {
  if [ "${REMOTE_CLUSTER}" = "true" ]; then
    ssh -i "${REMOTE_SSH_KEY}" "matt@${REMOTE_MASTER}" "$@"
  else
    "$@"
  fi
}

# Function to copy file to remote cluster
copy_to_remote() {
  local src="$1"
  local dest="$2"
  if [ "${REMOTE_CLUSTER}" = "true" ]; then
    scp -i "${REMOTE_SSH_KEY}" "${src}" "matt@${REMOTE_MASTER}:${dest}"
  fi
}

# Set Kubernetes context if provided
if [ -n "$K8S_CONTEXT" ]; then
  kubectl config use-context "$K8S_CONTEXT"
fi

# Setup for Minikube
if [ "${USE_MINIKUBE}" = "true" ]; then
  echo "Setting up for Minikube deployment..."
  
  # Point Docker client to Minikube's Docker daemon
  eval $(minikube docker-env)
  
  # Force local image mode
  USE_LOCAL_IMAGE=true
  
  # Use Minikube's built-in registry
  IMAGE_REGISTRY="docker.io/library"
fi

FULL_IMAGE_NAME="${IMAGE_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"

# If using local image, we don't need a registry prefix for minikube
if [ "${USE_LOCAL_IMAGE}" = "true" ]; then
  if [ "${USE_MINIKUBE}" = "true" ]; then
    echo "Using Minikube's Docker daemon for local image"
    FULL_IMAGE_NAME="${IMAGE_NAME}:${IMAGE_TAG}"
  else
    echo "Using local image without registry"
    FULL_IMAGE_NAME="${IMAGE_NAME}:${IMAGE_TAG}"
  fi
  
  # Build the image locally
  echo "Building Docker image: ${FULL_IMAGE_NAME}"
  docker build -t "${FULL_IMAGE_NAME}" -f Dockerfile.k8s .
  
  # If using remote cluster, we need to save the image and copy it to the remote server
  if [ "${REMOTE_CLUSTER}" = "true" ]; then
    echo "Saving Docker image to tar file..."
    docker save -o "${IMAGE_NAME}.tar" "${FULL_IMAGE_NAME}"
    
    echo "Copying Docker image to remote server..."
    copy_to_remote "${IMAGE_NAME}.tar" "/tmp/${IMAGE_NAME}.tar"
    
    echo "Loading Docker image on remote server..."
    run_remote_kubectl "docker load -i /tmp/${IMAGE_NAME}.tar && rm /tmp/${IMAGE_NAME}.tar"
  fi
fi

# Build the image if requested
if [ "${BUILD_IMAGE}" = "true" ] && [ "${USE_LOCAL_IMAGE}" != "true" ]; then
  echo "Building Docker image..."
  ./scripts/build-k8s-image.sh --registry "${IMAGE_REGISTRY}" --tag "${IMAGE_TAG}" --push
fi

# Replace image registry and tag placeholders in Kubernetes manifests
echo "Preparing Kubernetes manifests..."
for file in kubernetes/*.yaml; do
  # Replace namespace placeholder
  sed -i "s|\${NAMESPACE}|${NAMESPACE}|g" "$file"
  
  if [[ -f "$file" && "$file" == *deployment.yaml ]]; then
    # For deployment files, we need to update the image reference
    if [ "${USE_LOCAL_IMAGE}" = "true" ]; then
      # For local image, use the full image name
      sed -i "s|image:.*|image: ${FULL_IMAGE_NAME}|g" "$file"
      
      # Update imagePullPolicy if using local image
      if [ "${USE_MINIKUBE}" = "true" ] || [ "${REMOTE_CLUSTER}" = "true" ]; then
        sed -i "s|imagePullPolicy:.*|imagePullPolicy: Never|g" "$file"
      else
        sed -i "s|imagePullPolicy:.*|imagePullPolicy: IfNotPresent|g" "$file"
      fi
    else
      # For remote registry
      sed -i "s|image:.*|image: ${FULL_IMAGE_NAME}|g" "$file"
    fi
  fi
done

# If using remote cluster, copy all kubernetes manifests to the remote server
if [ "${REMOTE_CLUSTER}" = "true" ]; then
  echo "Copying Kubernetes manifests to remote server..."
  run_remote_kubectl "mkdir -p /tmp/icn-kubernetes"
  for file in kubernetes/*.yaml; do
    copy_to_remote "$file" "/tmp/icn-kubernetes/$(basename $file)"
  done
fi

# Function to apply kubernetes manifests
apply_kubernetes_manifests() {
  local namespace_yaml="kubernetes/namespace.yaml"
  local configmap_yaml="kubernetes/configmap.yaml"
  local pvc_yaml="kubernetes/persistent-volume-claims.yaml"
  local coop1_primary_deployment="kubernetes/coop1-primary-deployment.yaml"
  local coop1_primary_service="kubernetes/coop1-primary-service.yaml"
  local coop2_primary_deployment="kubernetes/coop2-primary-deployment.yaml"
  local coop2_primary_service="kubernetes/coop2-primary-service.yaml"
  local coop1_secondary_deployment="kubernetes/coop1-secondary-deployment.yaml"
  local coop1_secondary_service="kubernetes/coop1-secondary-service.yaml"
  local coop2_secondary_deployment="kubernetes/coop2-secondary-deployment.yaml"
  local coop2_secondary_service="kubernetes/coop2-secondary-service.yaml"
  
  if [ "${REMOTE_CLUSTER}" = "true" ]; then
    namespace_yaml="/tmp/icn-kubernetes/namespace.yaml"
    configmap_yaml="/tmp/icn-kubernetes/configmap.yaml"
    pvc_yaml="/tmp/icn-kubernetes/persistent-volume-claims.yaml"
    coop1_primary_deployment="/tmp/icn-kubernetes/coop1-primary-deployment.yaml"
    coop1_primary_service="/tmp/icn-kubernetes/coop1-primary-service.yaml"
    coop2_primary_deployment="/tmp/icn-kubernetes/coop2-primary-deployment.yaml"
    coop2_primary_service="/tmp/icn-kubernetes/coop2-primary-service.yaml"
    coop1_secondary_deployment="/tmp/icn-kubernetes/coop1-secondary-deployment.yaml"
    coop1_secondary_service="/tmp/icn-kubernetes/coop1-secondary-service.yaml"
    coop2_secondary_deployment="/tmp/icn-kubernetes/coop2-secondary-deployment.yaml"
    coop2_secondary_service="/tmp/icn-kubernetes/coop2-secondary-service.yaml"
  fi
  
  # Clean up existing deployment if requested
  if [ "${CLEAN_FIRST}" = "true" ]; then
    echo "Cleaning up existing deployment..."
    run_remote_kubectl "kubectl delete namespace \"${NAMESPACE}\" --ignore-not-found=true"
    
    # Wait to ensure the namespace is fully deleted
    echo "Waiting for namespace to be deleted..."
    run_remote_kubectl "kubectl wait --for=delete namespace/\"${NAMESPACE}\" --timeout=60s || true"
  fi
  
  # Create namespace if it doesn't exist
  echo "Creating Kubernetes namespace (if it doesn't exist)..."
  run_remote_kubectl "kubectl get namespace \"${NAMESPACE}\" || kubectl apply -f \"${namespace_yaml}\""
  
  # Apply ConfigMap first
  echo "Deploying ConfigMap..."
  run_remote_kubectl "kubectl apply -f \"${configmap_yaml}\""
  
  # Apply PersistentVolumeClaims
  echo "Creating persistent volume claims..."
  run_remote_kubectl "kubectl apply -f \"${pvc_yaml}\""
  
  # Deploy primary nodes first
  echo "Deploying primary nodes..."
  run_remote_kubectl "kubectl apply -f \"${coop1_primary_deployment}\""
  run_remote_kubectl "kubectl apply -f \"${coop1_primary_service}\""
  run_remote_kubectl "kubectl apply -f \"${coop2_primary_deployment}\""
  run_remote_kubectl "kubectl apply -f \"${coop2_primary_service}\""
  
  # Wait for primary nodes to be ready
  echo "Waiting for primary nodes to be ready..."
  run_remote_kubectl "kubectl rollout status deployment/coop1-primary -n \"${NAMESPACE}\""
  run_remote_kubectl "kubectl rollout status deployment/coop2-primary -n \"${NAMESPACE}\""
  
  # Deploy secondary nodes
  echo "Deploying secondary nodes..."
  run_remote_kubectl "kubectl apply -f \"${coop1_secondary_deployment}\""
  run_remote_kubectl "kubectl apply -f \"${coop1_secondary_service}\""
  run_remote_kubectl "kubectl apply -f \"${coop2_secondary_deployment}\""
  run_remote_kubectl "kubectl apply -f \"${coop2_secondary_service}\""
  
  # Wait for secondary nodes to be ready
  echo "Waiting for secondary nodes to be ready..."
  run_remote_kubectl "kubectl rollout status deployment/coop1-secondary -n \"${NAMESPACE}\""
  run_remote_kubectl "kubectl rollout status deployment/coop2-secondary -n \"${NAMESPACE}\""
}

# Apply Kubernetes manifests
apply_kubernetes_manifests

echo "ICN network has been deployed to Kubernetes!"
echo "To check the status, run: kubectl get pods -n ${NAMESPACE}"

# If using Minikube, show how to access the services
if [ "${USE_MINIKUBE}" = "true" ]; then
  echo ""
  echo "Since you're using Minikube, you can access the services with:"
  echo "  minikube service coop1-primary -n ${NAMESPACE}"
  echo "  minikube service coop2-primary -n ${NAMESPACE}"
fi

# Clean up temporary files if using remote cluster
if [ "${REMOTE_CLUSTER}" = "true" ]; then
  echo "Cleaning up temporary files on remote server..."
  run_remote_kubectl "rm -rf /tmp/icn-kubernetes"
  
  echo ""
  echo "To check the status on the remote cluster, run:"
  echo "  ssh -i ${REMOTE_SSH_KEY} matt@${REMOTE_MASTER} kubectl get pods -n ${NAMESPACE}"
fi 