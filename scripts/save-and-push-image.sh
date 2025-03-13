#!/bin/bash
set -e

echo "================================================================"
echo "Saving and Pushing Docker Image"
echo "================================================================"
echo "This script will save the Docker image locally and push it"
echo "from the remote server."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Build the image
echo "Building Docker image..."
docker build -t icn-test-node:latest -f Dockerfile.k8s .

# Save the image to a tar file
echo "Saving image to tar file..."
docker save icn-test-node:latest -o icn-test-node.tar

# Copy the tar file to the remote server
echo "Copying image to remote server..."
scp -i /home/matt/.ssh/id_rsa_new icn-test-node.tar matt@10.10.100.102:~/

# Load and push the image on the remote server
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Loading image on remote server...'
docker load -i ~/icn-test-node.tar

echo 'Tagging image for local registry...'
docker tag icn-test-node:latest localhost:30500/icn-test-node:latest

echo 'Pushing image to registry...'
docker push localhost:30500/icn-test-node:latest

echo 'Cleaning up...'
rm ~/icn-test-node.tar
"

# Clean up local tar file
echo "Cleaning up local tar file..."
rm icn-test-node.tar

echo "================================================================"
echo "Image pushed successfully."
echo "================================================================" 