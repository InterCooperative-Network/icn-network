#!/bin/bash

echo "Deploying ICN Network on Master Node Only"
echo "----------------------------------------"

# Create namespace if it doesn't exist
echo "Creating namespace icn-system if it doesn't exist..."
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "sudo kubectl create namespace icn-system --dry-run=client -o yaml | sudo kubectl apply -f -"

# Apply the master-only deployment
echo "Applying master-only deployment..."
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "sudo kubectl apply -f ~/icn-deploy/master-only-deployment.yaml"

# Wait for deployment to be ready
echo "Waiting for deployment to be ready..."
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "sudo kubectl rollout status deployment/icn-master-only -n icn-system --timeout=120s"

# Check the status of the deployment
echo "Checking status of the deployment..."
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "sudo kubectl get pods -n icn-system -o wide"

echo "ICN Network deployment on master node completed!" 