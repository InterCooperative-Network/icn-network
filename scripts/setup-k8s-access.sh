#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

echo "================================================================"
echo "Setting up Kubernetes Access for User"
echo "================================================================"
echo "This script will:"
echo "1. Connect to the remote server"
echo "2. Create a copy of the Kubernetes config readable by our user"
echo "3. Set up environment variables for kubectl to use this config"
echo ""
echo "When prompted:"
echo "1. Enter your SSH key passphrase"
echo "2. Enter sudo password when requested on the remote host"
echo ""
echo "================================================================"
echo "Connecting now..."

# Connect with pseudo-terminal allocation
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} '
# Create ~/.kube directory if it does not exist
mkdir -p ~/.kube

# Create a copy of the k3s.yaml file that our user can read
echo "Creating a readable copy of Kubernetes config..."
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config
sudo chown $(id -u):$(id -g) ~/.kube/config
sudo chmod 600 ~/.kube/config

# Update the server URL to use localhost instead of 127.0.0.1
# This is often necessary for k3s configurations
echo "Updating server URL in the config..."
sed -i "s/127.0.0.1/localhost/g" ~/.kube/config

# Test that it works
echo "Testing Kubernetes access..."
kubectl get nodes

if [ $? -eq 0 ]; then
  echo ""
  echo "SUCCESS! Kubernetes access has been set up."
  echo "You can now run kubectl commands without sudo."
  echo ""
  echo "Here are some commands to try:"
  echo "kubectl get nodes"
  echo "kubectl get namespaces"
  echo "kubectl get pods --all-namespaces"
  echo ""
  
  # Print some information about the cluster
  echo "Nodes in cluster:"
  kubectl get nodes
  
  echo ""
  echo "Namespaces in cluster:"
  kubectl get namespaces
  
  echo ""
  echo "Checking for ICN pods in all namespaces:"
  kubectl get pods --all-namespaces | grep -i icn
else
  echo ""
  echo "ERROR: Failed to set up Kubernetes access."
  echo "Please check the error messages above."
fi

# Add KUBECONFIG to .bashrc if not already there
if ! grep -q "export KUBECONFIG" ~/.bashrc; then
  echo "Adding KUBECONFIG to .bashrc..."
  echo "export KUBECONFIG=~/.kube/config" >> ~/.bashrc
  echo "You may need to run \"source ~/.bashrc\" to use kubectl in new sessions."
fi

echo ""
echo "Setup complete!"
'

echo "================================================================"
echo "Setup process completed."
echo "================================================================" 