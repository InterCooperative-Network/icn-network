# Intercooperative Network (ICN)

A decentralized infrastructure for cooperative economies, enabling resource sharing and collaborative governance across federated networks.

## Overview

The ICN is a peer-to-peer network designed to facilitate cooperation and economic interactions between cooperatives and solidarity economy organizations. Key components include:

1. **Decentralized Identity (DID)** - Identity management for cooperatives and their members
2. **Secure Messaging** - End-to-end encrypted communication between nodes
3. **Mutual Credit** - Economic exchanges without traditional currencies
4. **Resource Sharing** - Facilitating sharing of resources between cooperatives
5. **Governance** - Supporting democratic decision-making processes
6. **Federation Management** - Creation and management of federated networks

## Project Structure

```
icn/
├── crates/                    # Workspace crates
│   ├── core/                 # Core functionality and utilities
│   ├── network/              # Networking and peer-to-peer communication
│   │   └── overlay/          # Overlay network functionality
│   ├── storage/              # Storage system with distributed capabilities
│   │   ├── distributed/      # Distributed storage
│   │   └── federation/       # Federation storage routing  
│   ├── governance/           # Governance mechanisms
│   ├── economic/             # Economic and mutual credit models
│   ├── identity/             # Identity management (DIDs)
│   ├── federation/           # Federation management
│   ├── resource/             # Resource management
│   ├── dsl/                  # Domain-specific language
│   ├── vm/                   # Virtual machine for executing DSL
│   ├── node/                 # Node implementation
│   ├── cli/                  # Command-line interface
│   ├── crypto/               # Cryptographic utilities
│   ├── config/               # Configuration management
│   ├── integration/          # Integration tests and utilities
│   └── reputation/           # Reputation tracking and management
├── docs/                     # Documentation
├── examples/                 # Example implementations
├── tests/                    # Integration tests
├── scripts/                  # Utility scripts for development and deployment
├── kubernetes/               # Kubernetes deployment configurations
└── config/                   # Configuration files and templates
```

> **Note**: We've consolidated related crates to improve maintainability. Previously separate crates like `networking`, `distributed-storage`, and `federation-storage-router` have been integrated into the `network` and `storage` crates. See [docs/crate-consolidation.md](docs/crate-consolidation.md) for details.

## Features

- **Federation Management**: Create and manage federations of cooperative networks
- **Resource Sharing**: Efficient allocation and management of distributed resources
- **Governance**: Democratic decision-making and policy enforcement
- **Economic Models**: Support for various economic cooperation models
- **Identity Management**: Decentralized identity and access control
- **Distributed Storage**: Secure and efficient data storage across the network
- **IPv6-first Network Design**: Modern networking capabilities

## Architecture

The ICN implementation consists of several core components:

- **ICN Node** - The fundamental building block of the network, implemented in Rust
- **Identity Module** - Implements the DID (Decentralized Identity) specification
- **Networking Module** - Provides mesh networking capabilities
- **Storage Module** - Persistent storage of node data
- **Crypto Module** - Cryptographic primitives and confidential transactions
- **Economic Module** - Implements the mutual credit system

## Mutual Credit System

The mutual credit system is a core component of the ICN that enables economic exchanges between cooperatives without traditional currencies. Key features include:

- **Credit Accounts**: Each cooperative maintains a credit account with a defined credit limit
- **Transactions**: Secure, signed transactions between cooperatives
- **Balance Tracking**: Real-time balance tracking and transaction history
- **Credit Limits**: Configurable credit limits to manage risk
- **Transaction Verification**: Cryptographic verification of all transactions

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Cargo
- Git
- Docker (optional, for containerized deployment)
- Kubernetes (optional, for orchestrated deployment)

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/intercooperative-network/icn.git
   cd icn
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run tests:
   ```bash
   cargo test
   ```

### Running a Local Node

```bash
cargo run
```

### Usage

The ICN CLI provides a comprehensive interface to interact with the network:

```bash
# Join a federation
icn federation join <federation-id>

# Register a resource
icn resource register --name <n> --type <type> --capacity <capacity>

# Configure network
icn network configure --interface <interface> --mode <mode>
```

## Network Setup

We provide two methods to bootstrap the network:

1. **Standard Network** - Supports both IPv4 and IPv6
   ```bash
   bash scripts/bootstrap_network.sh icn-testnet 3
   bash scripts/start_network.sh
   ```

2. **IPv6-focused Network** - Prioritizes IPv6 connectivity
   ```bash
   bash scripts/bootstrap_ipv6_network.sh icn-testnet-ipv6 3
   bash scripts/start_ipv6_network.sh
   ```

## Development

The ICN project follows a modular architecture with clear separation of concerns:

- **Core**: Essential types and utilities
- **Network**: P2P communication and protocol implementation
- **Storage**: Distributed data storage and retrieval
- **Governance**: Decision-making and policy enforcement
- **Economic**: Resource allocation and economic models
- **Federation**: Federation management and coordination
- **Resource**: Resource tracking and allocation
- **Identity**: Identity management and access control

### Development Workflow

1. **Start the testnet** to create a running environment
2. **Develop features** in your local codebase
3. **Build and restart** the testnet to test your changes
4. **Monitor logs** to debug issues

### Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and development process.

## Deploying to Kubernetes

We provide configurations for deploying the ICN node to a Kubernetes cluster. See the `kubernetes/` directory for deployment manifests.

```bash
# Deploy using kubectl
kubectl apply -f kubernetes/namespace.yaml
kubectl apply -f kubernetes/configmap.yaml
kubectl apply -f kubernetes/persistent-volume-claims.yaml
kubectl apply -f kubernetes/coop1-primary-deployment.yaml
kubectl apply -f kubernetes/coop1-primary-service.yaml
```

## Documentation

- [Architecture Overview](docs/architecture/README.md)
- [API Documentation](docs/api/README.md)
- [User Guide](docs/user/README.md)
- [Development Guide](docs/development/README.md)
- [Testnet Setup](docs/testnet/README.md)

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- All contributors who have helped shape and improve this project
- The cooperative economy community for their valuable input and support
