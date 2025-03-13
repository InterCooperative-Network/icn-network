#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

# Print header
echo "================================================================"
echo "Remote Server Disk Space Check"
echo "================================================================"
echo "This script will connect to ${REMOTE_HOST} and check disk space."
echo ""
echo "When prompted, enter your SSH key passphrase."
echo "================================================================"

# Connect to the remote server and check disk space
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} "
  echo '=== Disk Space Usage ==='
  df -h
  
  echo '=== Docker Images ==='
  sudo docker images
  
  echo '=== Docker Disk Usage ==='
  sudo docker system df -v
  
  echo '=== Large Files ==='
  echo 'Top 15 largest directories in /var:'
  sudo du -h /var --max-depth=2 2>/dev/null | sort -rh | head -15
  
  echo '=== Node Status ==='
  sudo kubectl get nodes
  sudo kubectl describe nodes | grep -A 5 'Conditions:'
"

echo "================================================================"
echo "Disk space check completed."
echo "================================================================" 