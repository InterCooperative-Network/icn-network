#!/bin/bash

echo "Cleaning up old deployments in icn-system namespace"
echo "---------------------------------------------------"

# Connect to the server
ssh -t -i /home/matt/.ssh/id_rsa_new matt@10.10.100.102 "
    echo 'Deleting old deployments...'
    sudo kubectl delete deployment coop1-primary coop1-secondary coop2-primary coop2-secondary -n icn-system
    
    echo 'Deleting old services...'
    sudo kubectl delete service coop1-primary coop1-secondary coop2-primary coop2-secondary -n icn-system
    
    echo 'Checking remaining resources in icn-system namespace...'
    sudo kubectl get all -n icn-system
"

echo "Cleanup completed!" 