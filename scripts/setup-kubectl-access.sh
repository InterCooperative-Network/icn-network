#!/bin/bash
set -e

echo "================================================================"
echo "Setting up kubectl access for non-root user"
echo "================================================================"
echo "This script will connect to 10.10.100.102 and set up kubectl"
echo "access for the regular user by copying the k3s config file to"
echo "the user's home directory with appropriate permissions."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e
echo 'Creating .kube directory if it does not exist...'
mkdir -p ~/.kube

echo 'Copying k3s config file to user directory...'
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config

echo 'Setting proper ownership...'
sudo chown \$USER:\$USER ~/.kube/config

echo 'Setting restrictive permissions...'
chmod 600 ~/.kube/config

echo 'Updating server address in the config...'
sed -i 's/127.0.0.1/10.10.100.102/g' ~/.kube/config

echo 'Setting KUBECONFIG environment variable...'
export KUBECONFIG=~/.kube/config
echo 'export KUBECONFIG=~/.kube/config' >> ~/.bashrc

echo 'Testing kubectl access...'
kubectl get nodes
"

echo "================================================================"
echo "kubectl setup completed."
echo "================================================================" 