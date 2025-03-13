#!/bin/bash
set -e

echo "================================================================"
echo "Updating K3s Service Configuration"
echo "================================================================"
echo "This script will update the k3s service configuration to allow"
echo "non-root access by setting the --write-kubeconfig-mode option."
echo ""
echo "When prompted, enter your SSH key passphrase and sudo password."
echo "================================================================"

ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
set -e

# Update k3s service configuration
echo 'Updating k3s service configuration...'
sudo sed -i 's/ExecStart=\/usr\/local\/bin\/k3s server/ExecStart=\/usr\/local\/bin\/k3s server --write-kubeconfig-mode 644/g' /etc/systemd/system/k3s.service

# Restart k3s service
echo 'Restarting k3s service...'
sudo systemctl daemon-reload
sudo systemctl restart k3s

echo 'Waiting for k3s to restart...'
sleep 10

# Verify the service is running
echo 'Checking k3s service status...'
sudo systemctl status k3s --no-pager
"

echo "================================================================"
echo "K3s service configuration updated and service restarted."
echo "================================================================" 