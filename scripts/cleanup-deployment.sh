#!/bin/bash
set -e

echo "================================================================"
echo "Cleaning Up ICN Deployment"
echo "================================================================"
echo "This script will clean up any failed deployments and resources"
echo "in the icn-system namespace."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Deleting deployments...'
kubectl delete deployment --all -n icn-system || echo 'No deployments to delete'

echo 'Deleting services...'
kubectl delete service --all -n icn-system || echo 'No services to delete'

echo 'Deleting configmaps...'
kubectl delete configmap --all -n icn-system || echo 'No configmaps to delete'

echo 'Deleting pods...'
kubectl delete pod --all -n icn-system || echo 'No pods to delete'

echo 'Waiting for resources to be deleted...'
sleep 5

echo 'Current namespace status:'
kubectl get all -n icn-system
"

echo "================================================================"
echo "Cleanup completed."
echo "================================================================" 