#!/bin/bash
set -e

# Configuration
NAMESPACE="icn-system"
IMAGE_REGISTRY="10.10.100.102:30500"
IMAGE_NAME="icn-test-node"
IMAGE_TAG="latest"
FULL_IMAGE_REF="${IMAGE_REGISTRY}/${IMAGE_NAME}:${IMAGE_TAG}"

# Print header
echo "================================================================"
echo "Update Image References in Deployment Files"
echo "================================================================"
echo "This script will update all deployment YAML files to use:"
echo "Image: ${FULL_IMAGE_REF}"
echo "Namespace: ${NAMESPACE}"
echo "================================================================"

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
  sed -i "s/\${NAMESPACE}/${NAMESPACE}/g" "$file"
  
  # Update image references - specifically match the image: line in deployment files
  sed -i "s|image: icn-network:latest|image: ${FULL_IMAGE_REF}|g" "$file"
  
  echo "Updated $file"
done

# Also update any other YAML files to use the correct namespace
for file in kubernetes/*.yaml; do
  if [[ "$file" != *"deployment.yaml" ]] && [[ $(basename "$file") != "namespace.yaml" ]]; then
    # Update namespace references
    sed -i -E "s/namespace: icn-network-[0-9]+/namespace: ${NAMESPACE}/g" "$file"
    sed -i "s/\${NAMESPACE}/${NAMESPACE}/g" "$file"
    
    echo "Updated $file"
  fi
done

echo "================================================================"
echo "All deployment files have been updated."
echo "Next steps:"
echo "1. Transfer the files to the remote server: ./scripts/transfer-and-deploy.sh"
echo "2. Check the deployment status: ./scripts/check-icn-status.sh"
echo "================================================================" 