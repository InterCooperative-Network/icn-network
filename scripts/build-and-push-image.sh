#!/bin/bash
set -e

echo "================================================================"
echo "Building and Pushing Docker Image"
echo "================================================================"
echo "This script will build the Docker image and push it to the registry"
echo "at 10.10.100.102:30500."
echo "================================================================"

# Build the Docker image
echo "Building Docker image..."
docker build -t icn-test-node:latest -f Dockerfile.k8s .
echo "Docker image built successfully."

# Tag the image for the registry
echo "Tagging image for registry..."
docker tag icn-test-node:latest 10.10.100.102:30500/icn-test-node:latest
echo "Image tagged successfully."

# Push the image to the registry
echo "Pushing image to registry..."
docker push 10.10.100.102:30500/icn-test-node:latest
echo "Image pushed successfully."

echo "================================================================"
echo "Docker image build and push completed."
echo "================================================================" 