#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
NAMESPACE="icn-system"

echo "================================================================"
echo "Checking deployment status for namespace: $NAMESPACE"
echo "================================================================"
echo "When prompted, enter your SSH key passphrase."

# SSH to remote host and check status
ssh -i ${SSH_KEY} matt@${REMOTE_HOST} "
  # You don't need to setup kubeconfig, just try the default
  
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
  echo 'Logs from a primary node (if available):'
  PRIMARY_POD=\$(kubectl get pods -n ${NAMESPACE} -l role=primary -o name | head -1)
  if [ ! -z \"\$PRIMARY_POD\" ]; then
    kubectl logs \$PRIMARY_POD -n ${NAMESPACE} --tail=20
  else
    echo 'No primary pod with label role=primary found.'
    echo 'Trying to find pods with coop1-primary or coop2-primary in name:'
    PRIMARY_POD=\$(kubectl get pods -n ${NAMESPACE} | grep -E 'coop[1-2]-primary' | head -1 | awk '{print \$1}')
    if [ ! -z \"\$PRIMARY_POD\" ]; then
      echo \"Found pod: \$PRIMARY_POD\"
      kubectl logs \$PRIMARY_POD -n ${NAMESPACE} --tail=20
    else
      echo 'No primary pods found.'
    fi
  fi
"

echo "================================================================" 