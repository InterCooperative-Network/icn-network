#!/bin/bash

echo "================================================================"
echo "Building and Pushing Simple Docker Image"
echo "================================================================"
echo "This script will build a simple Docker image and push it"
echo "to the registry at 10.10.100.102:30500."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Build the Docker image
echo "Building Docker image..."
docker build -t icn-test-node:latest -f Dockerfile.simple .

# Save the Docker image to a tar file
echo "Saving Docker image to tar file..."
docker save icn-test-node:latest > icn-test-node.tar

# Copy the tar file to the remote server
echo "Copying Docker image to remote server..."
scp -i /home/matt/.ssh/id_rsa_new icn-test-node.tar matt@10.10.100.102:~/

# Load and push the image on the remote server
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Loading Docker image...'
sudo ctr -n k8s.io images import ~/icn-test-node.tar

echo 'Tagging Docker image for local registry...'
sudo ctr -n k8s.io images tag docker.io/library/icn-test-node:latest 10.10.100.102:30500/icn-test-node:latest

echo 'Pushing Docker image to registry...'
sudo ctr -n k8s.io images push --plain-http 10.10.100.102:30500/icn-test-node:latest

echo 'Cleaning up...'
rm ~/icn-test-node.tar

echo 'Image pushed successfully!'
"

echo "================================================================"
echo "Docker image built and pushed successfully!"
echo "Now you can run the deployment script again."
echo "================================================================" 