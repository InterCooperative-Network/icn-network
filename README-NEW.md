# Intercooperative Network (ICN) Implementation

This repository contains an implementation of the Intercooperative Network (ICN), a decentralized platform designed to enable cooperation and economic interactions between cooperatives and solidarity economy organizations.

## Overview

The ICN is a peer-to-peer network that facilitates:

1. **Decentralized Identity (DID)** - Identity management for cooperatives and their members
2. **Secure Messaging** - End-to-end encrypted communication between nodes
3. **Mutual Credit** - Economic exchanges without traditional currencies
4. **Resource Sharing** - Facilitating sharing of resources between cooperatives
5. **Governance** - Supporting democratic decision-making processes

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

### How It Works

1. **Account Creation**: Each cooperative creates a credit account with an agreed-upon credit limit
2. **Transaction Creation**: Cooperatives can create transactions to exchange credit
3. **Transaction Processing**: Transactions are verified and processed by the network
4. **Balance Updates**: Account balances are updated accordingly
5. **History Tracking**: All transactions are recorded and can be queried

### Example Usage

```rust
// Create a credit account
let account = economic.create_account(1000)?;

// Create a transaction
let transaction = economic.create_transaction(
    "did:icn:coop-2:node-1",
    100,
    Some("Payment for services".to_string()),
)?;

// Process a received transaction
economic.process_transaction(&received_transaction)?;

// Check balance
let balance = economic.get_balance("did:icn:coop-1:node-1")?;

// Get transaction history
let history = economic.get_transaction_history("did:icn:coop-1:node-1")?;
```

## Getting Started

### Prerequisites

- Rust 1.70 or higher
- Docker
- Kubernetes cluster

### Building from Source

```bash
cargo build --release
```

### Running a Local Node

```bash
cargo run
```

### Deploying to Kubernetes

We provide a script to build and deploy the ICN node to a Kubernetes cluster:

```bash
./scripts/build-and-deploy.sh
```

## Usage

Once your ICN node is running, you can:

1. **Create an Identity** - Generate a DID for your cooperative
2. **Connect to Peers** - Join the network and discover other cooperatives
3. **Exchange Resources** - Participate in the mutual credit system
4. **Participate in Governance** - Join decision-making processes

## Development

### Project Structure

```
├── src/
│   ├── main.rs         # Entry point for the ICN node
│   ├── config.rs       # Configuration handling
│   ├── identity.rs     # DID implementation
│   ├── networking.rs   # P2P networking
│   ├── storage.rs      # Persistent storage
│   ├── crypto.rs       # Cryptographic operations
│   └── economic.rs     # Mutual credit system
├── tests/
│   └── economic_tests.rs # Tests for mutual credit system
├── kubernetes/         # Kubernetes deployment files
└── scripts/            # Utility scripts
```

### Future Work

Our roadmap includes:

1. **Advanced Governance** - Implementing voting systems
2. **Economic Exchange** - Full mutual credit system
3. **Resource Matching** - Algorithms for optimal resource sharing
4. **UI Development** - User-friendly interfaces

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT OR Apache-2.0 license. 