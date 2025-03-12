#!/bin/bash
# ICN Testnet Launcher Script
# This script launches an ICN testnet with multiple nodes and federations

set -e

echo "Starting ICN Testnet..."

# Check if the testnet directory exists
if [ ! -d "./testnet" ]; then
    echo "Error: Testnet directory not found. Run ./scripts/setup_testnet.sh first."
    exit 1
fi

# Generate TLS certificates for the testnet
mkdir -p ./testnet/certs
echo "Generating TLS certificates..."
openssl req -x509 -newkey rsa:4096 -keyout ./testnet/certs/testnet_key.pem -out ./testnet/certs/testnet_cert.pem -days 365 -nodes -subj "/CN=icn-testnet/O=ICN Network/C=US" 2>/dev/null

# Export the certificate path for use by the testnet
export ICN_TLS_CERT_PATH="$(pwd)/testnet/certs/testnet_cert.pem"
export ICN_TLS_KEY_PATH="$(pwd)/testnet/certs/testnet_key.pem"

# Build the testnet example if needed
if [ ! -f "./target/debug/examples/icn_testnet" ]; then
    echo "Building icn_testnet example..."
    cargo build --example icn_testnet
fi

# Start the testnet nodes
echo "Starting testnet nodes..."

# Start the comprehensive testnet with all ICN systems
echo "Launching comprehensive ICN testnet..."
RUST_LOG=info ./target/debug/examples/icn_testnet > ./testnet/logs/testnet.log 2>&1 &
TESTNET_PID=$!

# Save the PID for later
echo $TESTNET_PID > ./testnet/testnet.pid

echo "Testnet started with PID $TESTNET_PID"
echo "Logs are available at ./testnet/logs/testnet.log"
echo ""
echo "To stop the testnet, run: ./scripts/stop_testnet.sh"

# Optional: Create a simple monitoring dashboard
echo "Creating monitoring dashboard..."
cat > ./testnet/dashboard.html << EOF
<!DOCTYPE html>
<html>
<head>
    <title>ICN Testnet Dashboard</title>
    <style>
        body { font-family: Arial, sans-serif; margin: 0; padding: 20px; }
        .dashboard { max-width: 1200px; margin: 0 auto; }
        .header { background: #333; color: white; padding: 10px; text-align: center; }
        .federation-section { margin-top: 20px; border: 1px solid #ddd; padding: 10px; border-radius: 5px; }
        .federation-header { background: #f5f5f5; padding: 10px; font-weight: bold; }
        .node-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 20px; margin-top: 20px; }
        .node-card { border: 1px solid #ddd; padding: 15px; border-radius: 5px; }
        .node-primary { background-color: #e8f4fe; }
        .node-secondary { background-color: #f9f9f9; }
        .status-online { color: green; }
        .status-offline { color: red; }
        .system-indicator { display: inline-block; margin-right: 5px; width: 12px; height: 12px; border-radius: 50%; }
        .system-active { background-color: green; }
        .system-inactive { background-color: red; }
    </style>
</head>
<body>
    <div class="dashboard">
        <div class="header">
            <h1>ICN Testnet Dashboard</h1>
            <p>Network: icn-testnet</p>
        </div>
        
        <h2>Federation Overview</h2>
        <div id="federations">
            <!-- Federations will be populated here -->
        </div>
    </div>
    <script>
        // Simple mock data for the dashboard
        const federations = [
            {
                id: "federation-0",
                name: "Federation One",
                description: "First test federation",
                coops: [
                    {
                        id: "coop-0-0",
                        name: "Cooperative 0-0",
                        nodes: [
                            { id: "node-0-0", type: "Primary", status: "Online", address: "127.0.0.1:9001", systems: { identity: true, network: true, economic: true } },
                            { id: "node-0-1", type: "Secondary", status: "Online", address: "127.0.0.1:9002", systems: { identity: true, network: true, economic: true } }
                        ]
                    },
                    {
                        id: "coop-0-1",
                        name: "Cooperative 0-1",
                        nodes: [
                            { id: "node-0-2", type: "Primary", status: "Online", address: "127.0.0.1:9003", systems: { identity: true, network: true, economic: true } },
                            { id: "node-0-3", type: "Secondary", status: "Online", address: "127.0.0.1:9004", systems: { identity: true, network: true, economic: true } }
                        ]
                    }
                ]
            },
            {
                id: "federation-1",
                name: "Federation Two",
                description: "Second test federation",
                coops: [
                    {
                        id: "coop-1-0",
                        name: "Cooperative 1-0",
                        nodes: [
                            { id: "node-1-0", type: "Primary", status: "Online", address: "127.0.0.1:9005", systems: { identity: true, network: true, economic: true } },
                            { id: "node-1-1", type: "Secondary", status: "Online", address: "127.0.0.1:9006", systems: { identity: true, network: true, economic: true } }
                        ]
                    },
                    {
                        id: "coop-1-1",
                        name: "Cooperative 1-1",
                        nodes: [
                            { id: "node-1-2", type: "Primary", status: "Online", address: "127.0.0.1:9007", systems: { identity: true, network: true, economic: true } },
                            { id: "node-1-3", type: "Secondary", status: "Online", address: "127.0.0.1:9008", systems: { identity: true, network: true, economic: true } }
                        ]
                    }
                ]
            }
        ];
        
        const federationsContainer = document.getElementById('federations');
        
        // Populate federations
        federations.forEach(federation => {
            const federationSection = document.createElement('div');
            federationSection.className = 'federation-section';
            
            // Federation header
            const federationHeader = document.createElement('div');
            federationHeader.className = 'federation-header';
            federationHeader.innerHTML = \`
                <h3>\${federation.name} (\${federation.id})</h3>
                <p>\${federation.description}</p>
            \`;
            federationSection.appendChild(federationHeader);
            
            // Cooperatives and nodes
            federation.coops.forEach(coop => {
                const coopSection = document.createElement('div');
                coopSection.innerHTML = \`<h4>\${coop.name} (\${coop.id})</h4>\`;
                
                const nodeGrid = document.createElement('div');
                nodeGrid.className = 'node-grid';
                
                coop.nodes.forEach(node => {
                    const nodeCard = document.createElement('div');
                    nodeCard.className = \`node-card node-\${node.type.toLowerCase()}\`;
                    
                    // Systems indicators
                    const systemsHtml = Object.entries(node.systems).map(([system, active]) => {
                        return \`
                            <div>
                                <span class="system-indicator \${active ? 'system-active' : 'system-inactive'}"></span>
                                \${system.charAt(0).toUpperCase() + system.slice(1)}: \${active ? 'Active' : 'Inactive'}
                            </div>
                        \`;
                    }).join('');
                    
                    nodeCard.innerHTML = \`
                        <h4>\${node.id}</h4>
                        <p><strong>Type:</strong> \${node.type}</p>
                        <p><strong>Address:</strong> \${node.address}</p>
                        <p><strong>Status:</strong> <span class="status-\${node.status.toLowerCase()}">\${node.status}</span></p>
                        <div class="systems">
                            <p><strong>Systems:</strong></p>
                            \${systemsHtml}
                        </div>
                    \`;
                    nodeGrid.appendChild(nodeCard);
                });
                
                coopSection.appendChild(nodeGrid);
                federationSection.appendChild(coopSection);
            });
            
            federationsContainer.appendChild(federationSection);
        });
        
        // Auto-refresh the dashboard
        setInterval(() => {
            // In a real implementation, we would fetch node statuses from the API
            console.log('Refreshing dashboard data...');
        }, 10000);
    </script>
</body>
</html>
EOF

echo "Dashboard created at ./testnet/dashboard.html"
echo "Open this file in a browser to view the testnet status" 