#!/bin/bash
# Bootstrap ICN Network
# This script initializes a new ICN network from scratch with the specified configuration.

set -e

# Configuration
NETWORK_NAME=${1:-"icn-testnet"}
NODE_COUNT=${2:-3}
BASE_PORT=9000
DATA_DIR="./data/icn"
CONFIG_DIR="./config"
LOG_DIR="./data/log/icn"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Print header
echo -e "${BLUE}======================================${NC}"
echo -e "${BLUE}ICN Network Bootstrap Script${NC}"
echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Network Name: ${NETWORK_NAME}${NC}"
echo -e "${GREEN}Number of Nodes: ${NODE_COUNT}${NC}"
echo -e "${GREEN}Base Port: ${BASE_PORT}${NC}"
echo -e "${GREEN}Data Directory: ${DATA_DIR}${NC}"
echo -e "${GREEN}Config Directory: ${CONFIG_DIR}${NC}"
echo -e "${BLUE}======================================${NC}"

# Check for required commands
for cmd in cargo jq openssl; do
    if ! command -v $cmd &> /dev/null; then
        echo -e "${RED}Error: $cmd command not found. Please install it before continuing.${NC}"
        exit 1
    fi
done

# Create required directories
echo -e "${YELLOW}Creating required directories...${NC}"
mkdir -p ${DATA_DIR}/{identity,ledger,governance,network,storage}
mkdir -p ${CONFIG_DIR}/nodes
mkdir -p ${LOG_DIR}

# Generate network configuration
echo -e "${YELLOW}Generating network configuration...${NC}"
cp ${CONFIG_DIR}/network.yaml ${CONFIG_DIR}/${NETWORK_NAME}.yaml

# Generate bootstrap peer list
BOOTSTRAP_PEERS="["
for i in $(seq 1 $NODE_COUNT); do
    NODE_ID=$(printf "node-%03d" $i)
    NODE_PORT=$((BASE_PORT + i))
    if [ $i -gt 1 ]; then
        BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS},"
    fi
    BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS}\"/ip4/127.0.0.1/tcp/${NODE_PORT}/p2p/\${${NODE_ID}_PEER_ID}\""
done
BOOTSTRAP_PEERS="${BOOTSTRAP_PEERS}]"

# Generate identity keys for each node
echo -e "${YELLOW}Generating identity keys for each node...${NC}"
PEER_IDS=()
for i in $(seq 1 $NODE_COUNT); do
    NODE_ID=$(printf "node-%03d" $i)
    NODE_PORT=$((BASE_PORT + i))
    
    echo -e "${GREEN}Generating keys for ${NODE_ID}...${NC}"
    
    # Generate ed25519 keys
    openssl genpkey -algorithm ed25519 -out ${DATA_DIR}/identity/${NODE_ID}.key
    openssl pkey -in ${DATA_DIR}/identity/${NODE_ID}.key -pubout -out ${DATA_DIR}/identity/${NODE_ID}.pub
    
    # Generate a deterministic peer ID from the public key (this is a placeholder - would use a proper function)
    # In a real system we'd generate peer IDs properly based on the public key
    PEER_ID=$(openssl rand -hex 16)
    PEER_IDS+=($PEER_ID)
    
    # Create node configuration
    NODE_CONFIG="${CONFIG_DIR}/nodes/${NODE_ID}.yaml"
    cat > ${NODE_CONFIG} <<EOF
# Node Configuration for ${NODE_ID}
node_id: "${NODE_ID}"
node_type: "full"
listen_addr: "/ip4/0.0.0.0/tcp/${NODE_PORT}"
peer_id: "${PEER_ID}"
data_dir: "${DATA_DIR}/${NODE_ID}"
log_dir: "${LOG_DIR}"
network_config_path: "${CONFIG_DIR}/${NETWORK_NAME}.yaml"
EOF

    echo -e "${GREEN}Created configuration for ${NODE_ID} at ${NODE_CONFIG}${NC}"
done

# Update bootstrap peers in network config with actual peer IDs
echo -e "${YELLOW}Updating network configuration with peer IDs...${NC}"

# Get the content of the network config file
NETWORK_CONFIG=$(cat ${CONFIG_DIR}/${NETWORK_NAME}.yaml)

# Replace bootstrap_peers placeholder with our list
NETWORK_CONFIG=${NETWORK_CONFIG/bootstrap_peers: \[\]/bootstrap_peers: ${BOOTSTRAP_PEERS}}

# Now replace each placeholder with the actual peer ID
for i in $(seq 1 $NODE_COUNT); do
    NODE_ID=$(printf "node-%03d" $i)
    PEER_ID=${PEER_IDS[$((i-1))]}
    PLACEHOLDER="\${${NODE_ID}_PEER_ID}"
    NETWORK_CONFIG=${NETWORK_CONFIG//$PLACEHOLDER/$PEER_ID}
done

# Write the updated content back to the file
echo "$NETWORK_CONFIG" > ${CONFIG_DIR}/${NETWORK_NAME}.yaml

echo -e "${GREEN}Updated network configuration with peer IDs${NC}"

# Setup the genesis configuration
echo -e "${YELLOW}Setting up genesis configuration...${NC}"
cat > ${CONFIG_DIR}/genesis.json <<EOF
{
  "network_name": "${NETWORK_NAME}",
  "creation_time": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "federation": {
    "id": "genesis-federation",
    "name": "Genesis Federation",
    "founding_members": [
EOF

for i in $(seq 1 $NODE_COUNT); do
    NODE_ID=$(printf "node-%03d" $i)
    PEER_ID=${PEER_IDS[$((i-1))]}
    NODE_PORT=$((BASE_PORT + i))
    
    # Add comma for all but the first entry
    if [ $i -gt 1 ]; then
        echo "      }," >> ${CONFIG_DIR}/genesis.json
    fi
    
    cat >> ${CONFIG_DIR}/genesis.json <<EOF
      {
        "id": "${NODE_ID}",
        "peer_id": "${PEER_ID}",
        "addresses": ["/ip4/127.0.0.1/tcp/${NODE_PORT}"],
        "roles": ["validator", "relay"],
        "voting_power": 1
EOF
done

# Close the last member and the array
echo "      }" >> ${CONFIG_DIR}/genesis.json
cat >> ${CONFIG_DIR}/genesis.json <<EOF
    ],
    "initial_governance_parameters": {
      "voting_period_seconds": 86400,
      "quorum_percentage": 66,
      "proposal_threshold": 1
    }
  },
  "initial_economic_parameters": {
    "initial_credit": 1000,
    "default_credit_limit": 5000,
    "transaction_fee_percentage": 0.5
  }
}
EOF

echo -e "${GREEN}Genesis configuration created at ${CONFIG_DIR}/genesis.json${NC}"

# Create launcher script with terminal detection
echo -e "${YELLOW}Creating launcher script...${NC}"
cat > ./start_network.sh <<EOF
#!/bin/bash
# Start ICN Network Nodes

# Kill any previous instances
pkill -f "icn-node" || true

# Start each node in a separate terminal - trying multiple terminal emulators
TERMINAL=""
if command -v gnome-terminal &> /dev/null; then
    TERMINAL="gnome-terminal --"
elif command -v xterm &> /dev/null; then
    TERMINAL="xterm -e"
elif command -v konsole &> /dev/null; then
    TERMINAL="konsole -e"
elif command -v terminal &> /dev/null; then
    TERMINAL="terminal -e"
else
    echo "No supported terminal emulator found. Starting nodes in background."
    TERMINAL=""
fi

# Start each node
EOF

for i in $(seq 1 $NODE_COUNT); do
    NODE_ID=$(printf "node-%03d" $i)
    cat >> ./start_network.sh <<EOF
if [ -n "\$TERMINAL" ]; then
    \$TERMINAL bash -c "cargo run --bin icn-node -- --config ${CONFIG_DIR}/nodes/${NODE_ID}.yaml; exec bash" &
else
    # Start in background if no terminal available
    cargo run --bin icn-node -- --config ${CONFIG_DIR}/nodes/${NODE_ID}.yaml > ${LOG_DIR}/${NODE_ID}.log 2>&1 &
fi
sleep 2
EOF
done

chmod +x ./start_network.sh

echo -e "${BLUE}======================================${NC}"
echo -e "${GREEN}Network bootstrap complete!${NC}"
echo -e "${GREEN}To start the network, run: bash ./start_network.sh${NC}"
echo -e "${BLUE}======================================${NC}" 