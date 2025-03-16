#!/bin/bash

# Generate certificates
/usr/local/bin/generate-certs.sh

# Create configuration file from template
if [ -f /etc/icn/node.yaml.template ]; then
    # Replace environment variables in the template
    envsubst < /etc/icn/node.yaml.template > /etc/icn/node.yaml
    echo "Configuration file created at /etc/icn/node.yaml"
    cat /etc/icn/node.yaml
else
    echo "Warning: Configuration template not found at /etc/icn/node.yaml.template"
fi

# Start the ICN node
echo "Starting ICN node..."
exec /usr/local/bin/icn-node 