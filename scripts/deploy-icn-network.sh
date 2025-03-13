#!/bin/bash
set -e

echo "================================================================"
echo "Deploying ICN Network"
echo "================================================================"
echo "This script will deploy the ICN network to the Kubernetes cluster"
echo "at 10.10.100.102."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

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

echo 'Cleaning up any existing PVCs in Pending state...'
for pvc in \$(sudo kubectl get pvc -n icn-system -o jsonpath='{.items[?(@.status.phase=="Pending")].metadata.name}'); do
    echo \"Deleting PVC \$pvc...\"
    sudo kubectl delete pvc \$pvc -n icn-system
done

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

echo 'Applying persistent volume claims...'
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
echo "ICN Network deployment completed!"
echo "To check the status, run: ./scripts/check-icn-status.sh"
echo "================================================================" 