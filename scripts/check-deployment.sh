#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

# Get namespace from command line or use default
if [ -z "$1" ]; then
  # Find the most recent icn-network namespace
  echo "No namespace specified. Checking the most recent ICN Network deployment..."
  echo "When prompted, enter your SSH key passphrase."
  NAMESPACE=$(ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "kubectl get namespaces | grep icn-network | sort -r | head -1 | awk '{print \$1}'")
  if [ -z "$NAMESPACE" ]; then
    echo "No ICN Network namespace found. Please specify a namespace."
    exit 1
  fi
else
  NAMESPACE="$1"
fi

echo "================================================================"
echo "Checking deployment status for namespace: $NAMESPACE"
echo "================================================================"
echo "When prompted, enter your SSH key passphrase."

# SSH to remote host and check status
ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "
  # Setup kubeconfig
  if [ -f ~/.kube/config ]; then
    export KUBECONFIG=~/.kube/config
  fi
  
  echo 'Namespace Info:'
  kubectl get namespace ${NAMESPACE}
  
  echo ''
  echo 'Pod Status:'
  kubectl get pods -n ${NAMESPACE} -o wide
  
  echo ''
  echo 'Service Status:'
  kubectl get services -n ${NAMESPACE}
  
  echo ''
  echo 'Deployment Status:'
  kubectl get deployments -n ${NAMESPACE}
  
  echo ''
  echo 'Events (most recent first):'
  kubectl get events --sort-by=.metadata.creationTimestamp -n ${NAMESPACE} | tail -10
  
  echo ''
  echo 'Logs from primary node (if available):'
  PRIMARY_POD=\$(kubectl get pods -n ${NAMESPACE} -l role=primary -o name | head -1)
  if [ ! -z \"\$PRIMARY_POD\" ]; then
    kubectl logs \$PRIMARY_POD -n ${NAMESPACE} --tail=20
  else
    echo 'No primary pod found.'
  fi
"

echo "================================================================" 