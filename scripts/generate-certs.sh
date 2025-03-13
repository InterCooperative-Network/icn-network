#!/bin/bash

# Create certificates directory if it doesn't exist
mkdir -p /etc/icn/certs

# Change to the certificates directory
cd /etc/icn/certs

# Generate CA key and certificate
openssl genrsa -out ca.key 2048
openssl req -x509 -new -nodes -key ca.key -sha256 -days 365 -out ca.crt -subj "/CN=ICN Root CA/O=ICN Network"

# Generate node key and CSR
openssl genrsa -out node.key 1704
openssl req -new -key node.key -out node.csr -subj "/CN=${ICN_NODE_ID}/O=ICN Network/OU=${ICN_COOP_ID}"

# Sign the node certificate with the CA
openssl x509 -req -in node.csr -CA ca.crt -CAkey ca.key -CAcreateserial -out node.crt -days 365 -sha256

# Remove the CSR file
rm node.csr

# List the generated certificates
echo "Certificates generated successfully in /etc/icn/certs:"
ls -la 