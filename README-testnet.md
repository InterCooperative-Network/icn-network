# ICN Testnet

This document outlines how to set up, run, and interact with the Intercooperative Network (ICN) testnet.

## Overview

The ICN testnet provides a local development and testing environment for the Intercooperative Network. It simulates a network of federated nodes, each representing cooperative entities, with the following core systems:

- **Identity System** (DIDs and verifiable credentials)
- **Networking Layer** (federated node communication)
- **Economic System** (mutual credit)
- **Federation System** (federation relationships and governance)

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [OpenSSL](https://www.openssl.org/) for TLS certificate generation
- [jq](https://stedolan.github.io/jq/download/) for JSON processing (optional)

## Quick Start

### Setup

1. Clone the repository:
   ```bash
   git clone https://github.com/your-repo/icn-network.git
   cd icn-network
   ```

2. Run the setup script:
   ```bash
   ./scripts/setup_testnet.sh
   ```

3. Build the testnet:
   ```bash
   cargo build --features testing --example icn_testnet
   ```

### Running the Testnet

1. Start the testnet:
   ```bash
   ./scripts/start_testnet.sh
   ```

2. Monitor the testnet:
   - View logs: `tail -f ./testnet/logs/testnet.log`
   - Open the dashboard: `./testnet/dashboard.html` in a web browser

3. Stop the testnet:
   ```bash
   ./scripts/stop_testnet.sh
   ```

## Testnet Architecture

The testnet consists of multiple federations, each with several cooperative nodes:

```
Testnet
├── Federation 1
│   ├── Cooperative 1-1
│   │   ├── Primary Node
│   │   └── Secondary Node
│   └── Cooperative 1-2
│       ├── Primary Node
│       └── Secondary Node
└── Federation 2
    └── Cooperative 2-1
        ├── Primary Node
        └── Secondary Node
```

Each node runs the following systems:
- Identity (DID Manager)
- Networking (Node with peer connections)
- Economic (Mutual Credit System)

## Interacting with the Testnet

### Using the API

You can interact with the testnet nodes via their HTTP API:

```bash
# Get node status
curl http://localhost:9001/status

# Create a DID
curl -X POST http://localhost:9001/did/create -d '{"name": "Alice", "coop_id": "coop-0-0"}'

# Check mutual credit balance
curl http://localhost:9001/credit/balance/account-federation-0
```

### Using the Dashboard

The testnet includes a simple web dashboard for monitoring node status. Open `./testnet/dashboard.html` in a web browser to access it.

## Customizing the Testnet

You can customize the testnet by editing the configuration files:

- `./testnet/testnet_config.json`: Define federations, cooperatives, and nodes
- `./testnet/configs/node_template.json`: Modify default node configuration
- `./testnet/configs/federation_template.json`: Modify default federation configuration

After modifying the configurations, run `./testnet/generate_configs.sh` to regenerate the node-specific configurations.

## Development Workflow

1. **Start the testnet** to create a running environment
2. **Develop features** in your local codebase
3. **Build and restart** the testnet to test your changes
4. **Monitor logs** to debug issues

## Troubleshooting

### Common Issues

- **Port conflicts**: If the default ports (9001-9010) are in use, modify the `BASE_PORT` constant in `examples/icn_testnet.rs`
- **Certificate errors**: If TLS connections fail, check the certificate generation in `start_testnet.sh`
- **Node discovery issues**: Ensure all nodes can communicate on localhost and there are no firewall issues

### Getting Help

If you encounter issues with the testnet, please:

1. Check the logs: `cat ./testnet/logs/testnet.log`
2. Run with verbose logging: `RUST_LOG=debug ./scripts/start_testnet.sh`
3. File an issue in the GitHub repository with details of your problem

## Contributing

We welcome contributions to improve the testnet! Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for guidelines on how to contribute.

## License

This project is licensed under the MIT OR Apache-2.0 license. 