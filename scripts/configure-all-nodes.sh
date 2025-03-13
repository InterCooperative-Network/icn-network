#!/bin/bash

echo "================================================================"
echo "Configuring All Nodes for HTTP Registry"
echo "================================================================"
echo "This script will configure all nodes to use HTTP for the registry"
echo "at 10.10.100.102:30500."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Connect to the remote server and configure all nodes
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Creating registry configuration for containerd on master node...'
sudo mkdir -p /etc/rancher/k3s/

cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
EOF

echo 'Copying registry configuration to worker nodes...'
for node in k8s-worker1 k8s-worker2; do
  echo \"Copying to \$node...\"
  sudo ssh \$node 'mkdir -p /etc/rancher/k3s/'
  sudo scp /etc/rancher/k3s/registries.yaml \$node:/etc/rancher/k3s/
done

echo 'Restarting k3s on all nodes...'
echo 'Restarting master node...'
sudo systemctl restart k3s
sleep 10

for node in k8s-worker1 k8s-worker2; do
  echo \"Restarting \$node...\"
  sudo ssh \$node 'systemctl restart k3s-agent'
done

sleep 20

echo 'Checking if nodes are ready...'
sudo kubectl get nodes
"

echo "================================================================"
echo "All nodes configuration completed!"
echo "Now you can run the deployment script again."
echo "================================================================" 