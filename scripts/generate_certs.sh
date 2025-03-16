#!/bin/bash
set -e

# Default values
CERT_DIR=${ICN_CERT_DIR:-/etc/icn/certs}
COOP_ID=${ICN_COOP_ID:-coop-0}
NODE_ID=${ICN_NODE_ID:-node-0}
DAYS_VALID=365

# Create certificate directory if it doesn't exist
mkdir -p "$CERT_DIR"

# Generate CA key and certificate if they don't exist
if [ ! -f "$CERT_DIR/ca.key" ]; then
    openssl genrsa -out "$CERT_DIR/ca.key" 4096
    openssl req -new -x509 -days $DAYS_VALID -key "$CERT_DIR/ca.key" -out "$CERT_DIR/ca.crt" \
        -subj "/CN=ICN Root CA/O=ICN Network/OU=$COOP_ID"
fi

# Generate node key
openssl genrsa -out "$CERT_DIR/node.key" 2048

# Generate node CSR
openssl req -new -key "$CERT_DIR/node.key" -out "$CERT_DIR/node.csr" \
    -subj "/CN=$NODE_ID/O=ICN Network/OU=$COOP_ID"

# Create config file for certificate extensions
cat > "$CERT_DIR/node.ext" << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
extendedKeyUsage = serverAuth, clientAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = $NODE_ID
DNS.3 = $NODE_ID.$COOP_ID
EOF

# Sign the certificate
openssl x509 -req -days $DAYS_VALID \
    -in "$CERT_DIR/node.csr" \
    -CA "$CERT_DIR/ca.crt" \
    -CAkey "$CERT_DIR/ca.key" \
    -CAcreateserial \
    -out "$CERT_DIR/node.crt" \
    -extfile "$CERT_DIR/node.ext"

# Clean up temporary files
rm "$CERT_DIR/node.csr" "$CERT_DIR/node.ext"

# Set proper permissions
chmod 644 "$CERT_DIR/ca.crt" "$CERT_DIR/node.crt"
chmod 600 "$CERT_DIR/ca.key" "$CERT_DIR/node.key"

echo "Certificates generated successfully in $CERT_DIR:"
ls -l "$CERT_DIR" 