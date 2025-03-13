#!/bin/bash

# Configuration
REMOTE_HOST="10.10.100.102"
SSH_KEY="/home/matt/.ssh/id_rsa_new"

echo "================================================================"
echo "Interactive Kubernetes Check Script"
echo "================================================================"
echo "This script will connect to the remote server with a pseudo-terminal"
echo "for interactive sudo access."
echo ""
echo "When prompted:"
echo "1. Enter your SSH key passphrase"
echo "2. Enter sudo password when requested on the remote host"
echo ""
echo "You'll be connected to the remote host. Run these commands:"
echo "- sudo kubectl get nodes"
echo "- sudo kubectl get namespaces"
echo "- sudo kubectl get pods --all-namespaces | grep -i icn"
echo "- sudo kubectl get pods -n icn-system"
echo ""
echo "Type 'exit' when done"
echo "================================================================"
echo "Connecting now..."

# Connect with pseudo-terminal allocation
ssh -t -i ${SSH_KEY} matt@${REMOTE_HOST} 