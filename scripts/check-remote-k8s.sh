#!/bin/bash

# This script is a simplified version to check the remote Kubernetes cluster

# Remote server details
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

# Print instructions
echo "This script will connect to the remote Kubernetes cluster"
echo "When prompted, enter the passphrase for your SSH key"
echo "----------------------------------------------------"
echo ""

# SSH to the remote host
ssh -i "$SSH_KEY" matt@"$REMOTE_HOST" "
  # Check for different kubeconfig locations
  if [ -f ~/.kube/config ]; then
    echo 'Using ~/.kube/config'
    export KUBECONFIG=~/.kube/config
  elif [ -r /etc/rancher/k3s/k3s.yaml ]; then
    echo 'Using /etc/rancher/k3s/k3s.yaml'
    export KUBECONFIG=/etc/rancher/k3s/k3s.yaml
  else
    echo 'No kubeconfig found. Trying kubectl anyway...'
  fi

  # Try to get nodes
  echo 'Nodes in the cluster:'
  kubectl get nodes && echo 'Successfully connected to Kubernetes cluster!' || echo 'Failed to access Kubernetes'
  
  echo ''
  echo 'Namespaces:'
  kubectl get namespaces | grep icn-network || echo 'No ICN network namespaces found'
  
  # Try to check for running pods
  echo ''
  echo 'Looking for ICN pods in any namespace:'
  kubectl get pods --all-namespaces | grep icn || echo 'No ICN pods found'
" 