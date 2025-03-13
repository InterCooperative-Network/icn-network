#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"
NAMESPACE="icn-system"

# Print header
echo "================================================================"
echo "ICN Network Status Check"
echo "================================================================"
echo "This script will connect to ${REMOTE_HOST} and check the status"
echo "of the ICN deployment in the ${NAMESPACE} namespace."
echo ""
echo "When prompted, enter your SSH key passphrase."
echo "================================================================"

# Connect to the remote server and check status
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  echo '=== Nodes ==='
  sudo kubectl get nodes
  
  echo '=== Pods ==='
  sudo kubectl get pods -n ${NAMESPACE}
  
  echo '=== Services ==='
  sudo kubectl get services -n ${NAMESPACE}
  
  echo '=== Events (most recent first) ==='
  sudo kubectl get events --sort-by=.metadata.creationTimestamp -n ${NAMESPACE} | tail -15
  
  # Check logs for any pod that's not Running
  echo '=== Checking logs for problematic pods ==='
  for pod in \$(sudo kubectl get pods -n ${NAMESPACE} -o custom-columns=NAME:.metadata.name,STATUS:.status.phase | grep -v 'Running' | awk '{if(NR>1)print \$1}'); do
    if [ ! -z \"\$pod\" ]; then
      echo \"Logs for \$pod:\"
      sudo kubectl logs \$pod -n ${NAMESPACE} --tail=50 || echo \"Could not get logs\"
      echo \"---\"
    fi
  done
  
  echo '=== Pod descriptions ==='
  for pod in \$(sudo kubectl get pods -n ${NAMESPACE} -o custom-columns=NAME:.metadata.name | awk '{if(NR>1)print \$1}' | head -2); do
    echo \"Description for \$pod:\"
    sudo kubectl describe pod \$pod -n ${NAMESPACE} | grep -A 10 \"Events:\" || echo \"Could not get description\"
    echo \"---\"
  done
"

echo "================================================================"
echo "Status check completed."
echo "================================================================" 