#!/bin/bash

echo "================================================================"
echo "Configuring All Nodes for HTTP Registry (Using IPs)"
echo "================================================================"
echo "This script will configure all nodes to use HTTP for the registry"
echo "at 10.10.100.102:30500."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Connect to the remote server and configure all nodes
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Getting worker node IPs...'
WORKER1_IP=\$(sudo kubectl get nodes k8s-worker1 -o jsonpath='{.status.addresses[?(@.type==\"InternalIP\")].address}')
WORKER2_IP=\$(sudo kubectl get nodes k8s-worker2 -o jsonpath='{.status.addresses[?(@.type==\"InternalIP\")].address}')

echo \"Worker 1 IP: \$WORKER1_IP\"
echo \"Worker 2 IP: \$WORKER2_IP\"

echo 'Creating registry configuration for containerd on master node...'
sudo mkdir -p /etc/rancher/k3s/

cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  '10.10.100.102:30500':
    endpoint:
      - 'http://10.10.100.102:30500'
EOF

echo 'Copying registry configuration to worker nodes...'
for node_ip in \$WORKER1_IP \$WORKER2_IP; do
  echo \"Copying to \$node_ip...\"
  sudo ssh \$node_ip 'mkdir -p /etc/rancher/k3s/'
  sudo scp /etc/rancher/k3s/registries.yaml \$node_ip:/etc/rancher/k3s/
done

echo 'Restarting k3s on all nodes...'
echo 'Restarting master node...'
sudo systemctl restart k3s
sleep 10

for node_ip in \$WORKER1_IP \$WORKER2_IP; do
  echo \"Restarting k3s-agent on \$node_ip...\"
  sudo ssh \$node_ip 'systemctl restart k3s-agent'
done

sleep 20

echo 'Checking if nodes are ready...'
sudo kubectl get nodes
"

echo "================================================================"
echo "All nodes configuration completed!"
echo "Now you can run the deployment script again."
echo "================================================================" 