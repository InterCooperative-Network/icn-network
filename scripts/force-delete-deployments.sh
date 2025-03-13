#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
NAMESPACE="icn-system"

# Print header
echo "================================================================"
echo "Force Delete ICN Deployments"
echo "================================================================"
echo "This script will connect to ${REMOTE_HOST} and remove all"
echo "existing ICN deployments in the ${NAMESPACE} namespace."
echo ""
echo "WARNING: This will DELETE ALL deployments in the ${NAMESPACE} namespace!"
echo "Only PVCs will be preserved."
echo ""
echo "When prompted, enter your SSH key passphrase."
echo "================================================================"

# Confirm before proceeding
read -p "Continue with force deletion? (y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
  echo "Operation aborted."
  exit 1
fi

# Connect to the remote server and clean up the deployment
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  echo 'Force deleting ICN resources...'
  
  # Delete all deployments in the namespace
  echo 'Deleting all deployments in namespace...'
  sudo kubectl delete deployment --all -n ${NAMESPACE}
  
  # Delete all services in the namespace
  echo 'Deleting all services in namespace except system ones...'
  for svc in \$(sudo kubectl get services -n ${NAMESPACE} -o name | grep -v kubernetes); do
    sudo kubectl delete \$svc -n ${NAMESPACE}
  done
  
  # Delete all replicasets in the namespace
  echo 'Deleting all replicasets in namespace...'
  sudo kubectl delete replicaset --all -n ${NAMESPACE}
  
  # Delete all pods in the namespace
  echo 'Deleting all pods in namespace...'
  sudo kubectl delete pod --all -n ${NAMESPACE} --force --grace-period=0
  
  # Untaint the nodes with disk pressure
  echo 'Removing disk-pressure taint from nodes...'
  for node in \$(sudo kubectl get nodes -o=name | grep -i master); do
    sudo kubectl taint \$node node.kubernetes.io/disk-pressure- || echo 'No taint to remove'
  done
  
  # Wait for all resources to be deleted
  echo 'Waiting for resources to be deleted...'
  sleep 10
  
  echo 'Current status of namespace:'
  sudo kubectl get all -n ${NAMESPACE}
  
  echo 'Persistent Volume Claims (preserved):'
  sudo kubectl get pvc -n ${NAMESPACE}
"

echo "================================================================"
echo "Force deletion completed."
echo "You can now deploy the updated configuration."
echo "================================================================" 