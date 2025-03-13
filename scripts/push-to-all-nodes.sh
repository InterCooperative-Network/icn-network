#!/bin/bash

echo "================================================================"
echo "Pushing Docker Image to All Nodes"
echo "================================================================"
echo "This script will push the Docker image directly to each node."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

# Build the Docker image
echo "Building Docker image..."
docker build -t icn-test-node:latest -f Dockerfile.simple .

# Save the Docker image to a tar file
echo "Saving Docker image to tar file..."
docker save icn-test-node:latest > icn-test-node.tar

# Copy the tar file to the remote server
echo "Copying Docker image to remote server..."
scp -i /home/matt/.ssh/id_rsa_new icn-test-node.tar matt@10.10.100.102:~/

# Load the image on all nodes
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

echo 'Getting worker node IPs...'
WORKER1_IP=\$(sudo kubectl get nodes k8s-worker1 -o jsonpath='{.status.addresses[?(@.type==\"InternalIP\")].address}')
WORKER2_IP=\$(sudo kubectl get nodes k8s-worker2 -o jsonpath='{.status.addresses[?(@.type==\"InternalIP\")].address}')

echo \"Worker 1 IP: \$WORKER1_IP\"
echo \"Worker 2 IP: \$WORKER2_IP\"

echo 'Loading Docker image on master node...'
sudo ctr -n k8s.io images import ~/icn-test-node.tar

echo 'Tagging Docker image for local registry...'
sudo ctr -n k8s.io images tag docker.io/library/icn-test-node:latest 10.10.100.102:30500/icn-test-node:latest

echo 'Pushing Docker image to registry...'
sudo ctr -n k8s.io images push --plain-http 10.10.100.102:30500/icn-test-node:latest

echo 'Copying image to worker nodes...'
for node_ip in \$WORKER1_IP \$WORKER2_IP; do
  echo \"Copying to \$node_ip...\"
  # Create a script to load the image on the worker node
  cat << 'EOF' > ~/load-image.sh
#!/bin/bash
sudo ctr -n k8s.io images import /tmp/icn-test-node.tar
sudo ctr -n k8s.io images tag docker.io/library/icn-test-node:latest 10.10.100.102:30500/icn-test-node:latest
EOF
  chmod +x ~/load-image.sh
  
  # Copy the image and script to the worker node
  sudo scp ~/icn-test-node.tar \$node_ip:/tmp/
  sudo scp ~/load-image.sh \$node_ip:/tmp/
  
  # Run the script on the worker node
  sudo ssh \$node_ip 'bash /tmp/load-image.sh'
done

echo 'Cleaning up...'
rm ~/icn-test-node.tar ~/load-image.sh

echo 'Image pushed to all nodes successfully!'
"

echo "================================================================"
echo "Docker image pushed to all nodes successfully!"
echo "Now you can run the deployment script again."
echo "================================================================" 