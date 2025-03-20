# Intercooperative Network (ICN)

A decentralized infrastructure for cooperative economies, enabling resource sharing and collaborative governance across federated networks.

## Project Structure

```
icn/
├── crates/                    # Workspace crates
│   ├── core/                 # Core functionality and utilities
│   ├── network/              # Networking and communication
│   ├── storage/              # Distributed storage
│   ├── governance/           # Governance mechanisms
│   ├── economic/             # Economic models
│   ├── identity/             # Identity management
│   ├── federation/           # Federation management
│   ├── resource/             # Resource management
│   ├── dsl/                  # Domain-specific language
│   ├── vm/                   # Virtual machine
│   ├── node/                 # Node implementation
│   └── cli/                  # Command-line interface
├── docs/                     # Documentation
├── examples/                 # Example implementations
├── tests/                    # Integration tests
└── tools/                    # Development tools
```

## Features

- **Federation Management**: Create and manage federations of cooperative networks
- **Resource Sharing**: Efficient allocation and management of distributed resources
- **Governance**: Democratic decision-making and policy enforcement
- **Economic Models**: Support for various economic cooperation models
- **Identity Management**: Decentralized identity and access control
- **Distributed Storage**: Secure and efficient data storage across the network

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Cargo
- Git

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

### Usage

The ICN CLI provides a comprehensive interface to interact with the network:

```bash
# Join a federation
icn federation join <federation-id>

# Register a resource
icn resource register --name <name> --type <type> --capacity <capacity>

# Configure network
icn network configure --interface <interface> --mode <mode>
```

## Development

### Architecture

The ICN project follows a modular architecture with clear separation of concerns:

- **Core**: Essential types and utilities
- **Network**: P2P communication and protocol implementation
- **Storage**: Distributed data storage and retrieval
- **Governance**: Decision-making and policy enforcement
- **Economic**: Resource allocation and economic models
- **Federation**: Federation management and coordination
- **Resource**: Resource tracking and allocation
- **Identity**: Identity management and access control

### Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct and development process.

## Documentation

- [Architecture Overview](docs/architecture/README.md)
- [API Documentation](docs/api/README.md)
- [User Guide](docs/user/README.md)
- [Development Guide](docs/development/README.md)

## License

This project is licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Acknowledgments

- All contributors who have helped shape and improve this project
- The cooperative economy community for their valuable input and support
