#!/bin/bash

echo "================================================================"
echo "Building and Deploying ICN Node"
echo "================================================================"
echo "This script will build the ICN node image and deploy it to the"
echo "Kubernetes cluster at 10.10.100.102."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Build the Docker image
echo "Building Docker image..."
docker build -t icn-node:latest .

# Save the Docker image to a tar file
echo "Saving Docker image to tar file..."
docker save icn-node:latest > icn-node.tar

# Copy the tar file to the remote server
echo "Copying Docker image to remote server..."
scp -i /home/matt/.ssh/id_rsa_new icn-node.tar matt@10.10.100.102:~/

# Connect to the remote server to load and deploy the image
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

cd ~/

echo 'Loading Docker image...'
sudo ctr -n k8s.io images import ./icn-node.tar

echo 'Tagging Docker image for local registry...'
sudo ctr -n k8s.io images tag docker.io/library/icn-node:latest localhost:30500/icn-node:latest

echo 'Pushing Docker image to registry...'
sudo ctr -n k8s.io images push --plain-http localhost:30500/icn-node:latest

echo 'Cleaning up...'
rm ./icn-node.tar

echo 'Creating deployment directory...'
mkdir -p ~/icn-deploy
"

# Transfer Kubernetes deployment files
echo "Transferring Kubernetes deployment files..."
scp -i /home/matt/.ssh/id_rsa_new kubernetes/icn-deployment.yaml kubernetes/icn-pvc.yaml matt@10.10.100.102:~/icn-deploy/

# Deploy to Kubernetes
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

cd ~/icn-deploy

echo 'Creating namespace if it does not exist...'
sudo kubectl create namespace icn-system --dry-run=client -o yaml | sudo kubectl apply -f -

echo 'Applying persistent volume claims...'
sudo kubectl apply -f icn-pvc.yaml

echo 'Waiting for persistent volume claims to be created...'
sleep 5

echo 'Applying deployment...'
sudo kubectl apply -f icn-deployment.yaml

echo 'Waiting for deployment to be ready...'
sudo kubectl rollout status deployment/icn-node -n icn-system --timeout=300s

echo 'Checking deployment status...'
sudo kubectl get pods -n icn-system -o wide
"

echo "================================================================"
echo "ICN Node deployment completed!"
echo "================================================================" 