#!/bin/bash

echo "================================================================"
echo "Clean Deployment of ICN Network"
echo "================================================================"
echo "WARNING: This script will delete all existing PVCs in the icn-system"
echo "namespace and create new ones. All data will be lost."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

read -p "Are you sure you want to continue? (y/n) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    echo "Aborting."
    exit 1
fi

# Create deployment directory on remote server
ssh -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "mkdir -p ~/icn-deploy"
echo "Created deployment directory on remote server."

# Transfer all YAML files
echo "Transferring Kubernetes YAML files to remote server..."
scp -i /home/matt/.ssh/id_rsa_new kubernetes/*.yaml matt@10.10.100.102:~/icn-deploy/
echo "Files transferred successfully."

# Deploy to remote server
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

cd ~/icn-deploy

echo 'Checking if namespace icn-system exists...'
if ! sudo kubectl get namespace icn-system &>/dev/null; then
    echo 'Creating namespace icn-system...'
    sudo kubectl create -f namespace.yaml
else
    echo 'Namespace icn-system already exists.'
fi

echo 'Deleting all existing PVCs in icn-system namespace...'
sudo kubectl delete pvc --all -n icn-system

echo 'Ensuring local-path storage class exists...'
cat << 'EOF' | sudo kubectl apply -f -
apiVersion: storage.k8s.io/v1
kind: StorageClass
metadata:
  name: local-path
provisioner: rancher.io/local-path
volumeBindingMode: WaitForFirstConsumer
reclaimPolicy: Delete
EOF

echo 'Waiting for PVCs to be deleted...'
while [[ \$(sudo kubectl get pvc -n icn-system -o name | wc -l) -gt 0 ]]; do
    echo 'Waiting for PVCs to be deleted...'
    sleep 5
done

echo 'Creating new PVCs...'
sudo kubectl apply -f persistent-volume-claims.yaml

echo 'Applying ConfigMap...'
sudo kubectl apply -f configmap.yaml

echo 'Applying Services...'
sudo kubectl apply -f coop1-primary-service.yaml
sudo kubectl apply -f coop1-secondary-service.yaml
sudo kubectl apply -f coop2-primary-service.yaml
sudo kubectl apply -f coop2-secondary-service.yaml

echo 'Applying Deployments...'
sudo kubectl apply -f coop1-primary-deployment.yaml
sudo kubectl apply -f coop1-secondary-deployment.yaml
sudo kubectl apply -f coop2-primary-deployment.yaml
sudo kubectl apply -f coop2-secondary-deployment.yaml

echo 'Waiting for PVCs to be bound...'
for pvc in \$(sudo kubectl get pvc -n icn-system -o jsonpath='{.items[*].metadata.name}'); do
    echo \"Waiting for PVC \$pvc...\"
    while [[ \$(sudo kubectl get pvc \$pvc -n icn-system -o jsonpath='{.status.phase}') != 'Bound' ]]; do
        echo \"PVC \$pvc is not bound yet...\"
        sleep 5
    done
done

echo 'Waiting for deployments to be ready...'
for deployment in coop1-primary coop1-secondary coop2-primary coop2-secondary; do
    echo \"Waiting for \$deployment...\"
    sudo kubectl -n icn-system rollout status deployment/\$deployment --timeout=300s
done

echo 'Checking deployment status...'
sudo kubectl -n icn-system get pods
sudo kubectl -n icn-system get services
sudo kubectl -n icn-system get pvc
"

echo "================================================================"
echo "ICN Network clean deployment completed!"
echo "To check the status, run: ./scripts/check-icn-status.sh"
echo "================================================================" 