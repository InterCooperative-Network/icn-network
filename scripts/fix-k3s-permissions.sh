#!/bin/bash
set -e

echo "================================================================"
echo "Fixing K3s Permissions"
echo "================================================================"
echo "This script will fix the k3s permissions to allow non-root access"
echo "to kubectl commands."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Stopping k3s service...'
sudo systemctl stop k3s

echo 'Backing up existing k3s config...'
sudo cp /etc/rancher/k3s/k3s.yaml /etc/rancher/k3s/k3s.yaml.bak

echo 'Setting proper permissions on k3s directory...'
sudo chmod 755 /etc/rancher/k3s

echo 'Setting proper permissions on k3s config...'
sudo chmod 644 /etc/rancher/k3s/k3s.yaml

echo 'Creating .kube directory in home...'
mkdir -p ~/.kube

echo 'Copying k3s config to user directory...'
sudo cp /etc/rancher/k3s/k3s.yaml ~/.kube/config

echo 'Setting proper ownership of user config...'
sudo chown \$USER:\$USER ~/.kube/config

echo 'Setting proper permissions on user config...'
chmod 600 ~/.kube/config

echo 'Updating server address in config...'
sed -i 's/127.0.0.1/10.10.100.102/g' ~/.kube/config

echo 'Starting k3s service...'
sudo systemctl start k3s

echo 'Waiting for k3s to start...'
sleep 10

echo 'Testing kubectl access...'
kubectl get nodes

echo 'Setting KUBECONFIG environment variable...'
export KUBECONFIG=~/.kube/config
echo 'export KUBECONFIG=~/.kube/config' >> ~/.bashrc
"

echo "================================================================"
echo "K3s permissions fixed. You can now run kubectl commands."
echo "================================================================" 