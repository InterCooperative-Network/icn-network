#!/bin/bash
set -e

echo "================================================================"
echo "Setting Kubeconfig Permissions"
echo "================================================================"
echo "This script will manually set the permissions on the kubeconfig"
echo "file to ensure non-root access."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

# Set permissions on the kubeconfig file
echo 'Setting permissions on /etc/rancher/k3s/k3s.yaml...'
sudo chmod 644 /etc/rancher/k3s/k3s.yaml

# Copy the kubeconfig to the user's home directory
echo 'Copying kubeconfig to user directory...'
mkdir -p ~/.kube
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config

# Set ownership and permissions on the user's kubeconfig
echo 'Setting ownership and permissions on user kubeconfig...'
sudo chown \$USER:\$USER ~/.kube/config
chmod 600 ~/.kube/config

# Update the server address in the kubeconfig
echo 'Updating server address in kubeconfig...'
sed -i 's/127.0.0.1/10.10.100.102/g' ~/.kube/config

# Test kubectl access
echo 'Testing kubectl access...'
kubectl get nodes
"

echo "================================================================"
echo "Kubeconfig permissions set. You can now run kubectl commands."
echo "================================================================" 