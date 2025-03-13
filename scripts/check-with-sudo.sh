#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
NAMESPACE="icn-system"

echo "================================================================"
echo "Checking Kubernetes cluster status using sudo"
echo "================================================================"
echo "When prompted, enter your SSH key passphrase."
echo "You may also be prompted for the sudo password on the remote host."

# SSH to remote host and check status
ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "
  echo 'Available namespaces:'
  sudo kubectl get namespaces
  
  echo ''
  echo 'Checking ICN-related pods in all namespaces:'
  sudo kubectl get pods --all-namespaces | grep -i icn
  
  echo ''
  if [ -n \"${NAMESPACE}\" ]; then
    echo 'Pods in ${NAMESPACE} namespace:'
    sudo kubectl get pods -n ${NAMESPACE}
    
    echo ''
    echo 'Services in ${NAMESPACE} namespace:'
    sudo kubectl get services -n ${NAMESPACE}
  fi
  
  echo ''
  echo 'Node status:'
  sudo kubectl get nodes
  
  echo ''
  echo 'Testing if we can run our own kubectl commands:'
  if [ -f /etc/rancher/k3s/k3s.yaml ]; then
    echo 'Found k3s configuration, trying to use it with sudo...'
    sudo kubectl --kubeconfig=/etc/rancher/k3s/k3s.yaml get nodes
  fi
"

echo "================================================================" 