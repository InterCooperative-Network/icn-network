# ICN Network

The Intercooperative Network (ICN) is a comprehensive technical infrastructure designed to enable a cooperative-based dual power system. By integrating democratic governance, non-extractive economics, and cutting-edge privacy tools, ICN reimagines how communities, cooperatives, and federations organize economic, social, and political life.

## Project Overview

ICN is built as a modular, component-based system with a focus on:
- **Decentralized Identity**: DIDs and verifiable credentials
- **Federation-First Design**: Local autonomy with collaborative capabilities  
- **Democratic Governance**: Liquid democracy with multiple voting methods
- **Non-Extractive Economics**: Mutual credit systems that keep value within networks
- **Privacy and Security**: Zero-knowledge proofs, ring signatures, and secure multi-party computation

## Architecture

The project is organized into a modular architecture:

- **Core Layer**: Consensus, state management, cryptography, and common utilities
- **Service Layer**: Identity, governance, economic, and resource coordination systems
- **Application Layer**: User interfaces, APIs, and developer tools

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Docker and Docker Compose (for development environment)

### Building from Source

```bash
# Clone the repository
git clone https://github.com/intercoin/icn.git
cd icn

# Build the project
cargo build

# Run tests
cargo test
```

### Using Docker

```bash
# Start the development environment
docker-compose up -d

# Access the development shell
docker-compose exec icn-dev bash
```

## Project Status

This project is currently in early development, following the phased implementation approach outlined in the whitepaper:

- **Phase 1**: Foundation Layer (Core Infrastructure)
- **Phase 2**: Pilot-Ready System
- **Phase 3**: Cooperative Network
- **Phase 4**: Revolutionary Platform

See the [implementation roadmap](dev-docs/implementation-phases.mermaid) for more details.

## Contributing

We welcome contributions from developers, cooperatives, researchers, and community leaders. Please see [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

This project is licensed under the MIT License or Apache License 2.0, at your option - see the LICENSE files for details.
