#!/bin/bash
# ICN Testnet Setup Script
# This script prepares the environment for running an ICN testnet

set -e

echo "Setting up ICN Testnet environment..."

# Create directories
echo "Creating testnet directories..."
mkdir -p ./testnet/configs
mkdir -p ./testnet/logs
mkdir -p ./testnet/data

# Create configuration templates
echo "Creating configuration templates..."
cat > ./testnet/configs/node_template.json << EOF
{
  "listen_addr": "127.0.0.1:PORT",
  "peers": [],
  "node_id": "NODE_ID",
  "coop_id": "COOP_ID",
  "node_type": "NODE_TYPE",
  "discovery_interval": 30,
  "health_check_interval": 10
}
EOF

cat > ./testnet/configs/federation_template.json << EOF
{
  "federation_id": "FEDERATION_ID",
  "name": "FEDERATION_NAME",
  "description": "Federation description",
  "bootstrap_nodes": [],
  "governance_model": "direct",
  "membership_policy": "open"
}
EOF

# Create testnet configuration
echo "Creating testnet configuration..."
cat > ./testnet/testnet_config.json << EOF
{
  "network_name": "icn-testnet",
  "federations": [
    {
      "federation_id": "fed-1",
      "name": "Federation One",
      "description": "First test federation",
      "coops": [
        {
          "coop_id": "coop-1-1",
          "name": "Cooperative 1-1",
          "nodes": [
            {"node_id": "node-1-1-1", "node_type": "Primary", "port": 9001},
            {"node_id": "node-1-1-2", "node_type": "Secondary", "port": 9002}
          ]
        },
        {
          "coop_id": "coop-1-2",
          "name": "Cooperative 1-2",
          "nodes": [
            {"node_id": "node-1-2-1", "node_type": "Primary", "port": 9003},
            {"node_id": "node-1-2-2", "node_type": "Secondary", "port": 9004}
          ]
        }
      ]
    },
    {
      "federation_id": "fed-2",
      "name": "Federation Two",
      "description": "Second test federation",
      "coops": [
        {
          "coop_id": "coop-2-1",
          "name": "Cooperative 2-1",
          "nodes": [
            {"node_id": "node-2-1-1", "node_type": "Primary", "port": 9005},
            {"node_id": "node-2-1-2", "node_type": "Secondary", "port": 9006}
          ]
        }
      ]
    }
  ]
}
EOF

# Create monitoring dashboard config
echo "Creating monitoring configuration..."
cat > ./testnet/prometheus.yml << EOF
global:
  scrape_interval: 15s

scraping_configs:
  - job_name: 'icn-testnet'
    static_configs:
      - targets: ['localhost:9090']
EOF

# Create script to generate node configurations
cat > ./testnet/generate_configs.sh << 'EOF'
#!/bin/bash
# Generate node configurations based on testnet_config.json

CONFIG_FILE="./testnet_config.json"
OUTPUT_DIR="./configs"

echo "Generating node configurations from $CONFIG_FILE..."

# TODO: Add jq-based parsing of testnet_config.json to generate individual node configs
# For now, this is a placeholder

echo "Configuration generation complete."
EOF

chmod +x ./testnet/generate_configs.sh

echo "ICN Testnet setup complete!"
echo "Next steps:"
echo "1. Run 'cargo build --features testing' to build the testnet components"
echo "2. Run './scripts/start_testnet.sh' to start the testnet" 