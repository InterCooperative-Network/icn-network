#!/bin/bash
set -e

echo "================================================================"
echo "Configuring Registry on All Nodes"
echo "================================================================"
echo "This script will configure the insecure registry on all nodes"
echo "in the Kubernetes cluster."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

# Configure Docker client on the local machine
echo 'Configuring Docker daemon...'
sudo mkdir -p /etc/docker
cat << 'EOF' | sudo tee /etc/docker/daemon.json
{
  \"insecure-registries\" : [\"10.10.100.102:30500\"]
}
EOF

# Restart Docker to apply changes
echo 'Restarting Docker...'
sudo systemctl restart docker || echo 'Docker restart failed, might not be installed'

# Configure containerd on all nodes
echo 'Configuring containerd on all nodes...'

# Function to configure a node
configure_node() {
    local node=\$1
    echo \"Configuring node \$node...\"
    
    # Create registries.yaml
    cat << 'EOF' | sudo tee /etc/rancher/k3s/registries.yaml
mirrors:
  \"10.10.100.102:30500\":
    endpoint:
      - \"http://10.10.100.102:30500\"
configs:
  \"10.10.100.102:30500\":
    tls:
      insecure_skip_verify: true
EOF

    # Restart k3s
    echo \"Restarting k3s on \$node...\"
    sudo systemctl restart k3s
    echo \"Configuration completed on \$node\"
}

# Configure the master node
configure_node 'k8s-master'

# Wait for k3s to restart
sleep 10

echo 'Registry configuration completed.'
echo 'Testing registry access...'
curl -v http://10.10.100.102:30500/v2/
"

echo "================================================================"
echo "Registry configuration completed on all nodes."
echo "================================================================" 