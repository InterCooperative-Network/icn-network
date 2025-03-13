#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

# Print header
echo "================================================================"
echo "Remote Server Disk Space Cleanup"
echo "================================================================"
echo "This script will connect to ${REMOTE_HOST} and clean up disk space."
echo ""
echo "When prompted, enter your SSH key passphrase."
echo "================================================================"

# Connect to the remote server and clean up disk space
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  echo '=== Before Cleanup ==='
  df -h /
  
  echo '=== Cleaning APT Cache ==='
  sudo apt-get clean
  sudo apt-get autoremove -y
  
  echo '=== Cleaning Journal Logs ==='
  sudo journalctl --vacuum-time=1d
  
  echo '=== Cleaning Old Docker Images & Build Cache ==='
  # Prune unused containers
  sudo docker container prune -f
  
  # Prune unused volumes
  sudo docker volume prune -f
  
  # Prune build cache
  sudo docker builder prune -f
  
  # Force cleanup of untaged/dangling images
  sudo docker image prune -f
  
  echo '=== Removing Large Files in /tmp ==='
  sudo find /tmp -type f -size +50M -exec rm -f {} \\;
  
  echo '=== Cleaning Old Log Files ==='
  sudo find /var/log -type f -name \"*.gz\" -delete
  sudo find /var/log -type f -name \"*.old\" -delete
  sudo find /var/log -type f -name \"*.1\" -delete
  sudo find /var/log -type f -name \"*.2\" -delete
  sudo find /var/log -type f -name \"*.3\" -delete
  
  echo '=== After Cleanup ==='
  df -h /
  
  echo '=== Node Status ==='
  sudo kubectl get nodes
"

echo "================================================================"
echo "Disk space cleanup completed."
echo "================================================================" 