# Intercooperative Network (ICN)

The Intercooperative Network (ICN) is a decentralized platform designed to enable cooperation and economic interactions between cooperatives and solidarity economy organizations.

## Project Overview

The ICN project aims to create a decentralized infrastructure that supports:

1. **Decentralized Identity (DID)**: Allowing cooperatives to establish and manage their digital identities
2. **Mutual Credit Systems**: Enabling economic exchanges between cooperatives without relying on traditional currencies
3. **Resource Sharing**: Facilitating the sharing of resources between cooperatives
4. **Governance**: Supporting democratic decision-making processes

## Repository Structure

The project is organized into modular components:

```
crates/
├── core/
│   ├── icn-common/       # Common utilities and types
│   ├── icn-crypto/       # Cryptographic primitives
├── identity/
│   ├── icn-did/          # Decentralized Identity implementation
│   ├── icn-credentials/  # Verifiable Credentials
├── economic/
│   ├── icn-mutual-credit/ # Mutual Credit implementation
├── storage/
│   ├── icn-storage-system/ # Storage system
examples/                  # Integration examples
standalone/               # Standalone example implementation
```

## Current Status

The project is in early development, with the following components implemented:

- **Core Utilities**: Basic cryptographic primitives and common types
- **Identity System**: DID implementation with local resolution
- **Mutual Credit**: Basic mutual credit system with account management and transactions
- **Integration Example**: Demonstrating the interaction between identity and mutual credit systems

## Getting Started

### Running the Standalone Example

For a quick demonstration of the core concepts:

```bash
cd standalone
cargo run
```

This example demonstrates the integration between the identity system and mutual credit system in a simplified manner.

### Running the Integration Example

To run the full integration example (requires all dependencies):

```bash
cargo run --example identity_and_credit
```

## Development Roadmap

The project is following a phased development approach:

1. **Phase 1 (Current)**: Core infrastructure and basic implementations
   - Identity system with DIDs
   - Basic mutual credit implementation
   - Integration between components

2. **Phase 2 (Planned)**: Enhanced functionality
   - Federation of DIDs across networks
   - Advanced credit policies and governance
   - Resource sharing mechanisms

3. **Phase 3 (Future)**: Network deployment
   - Decentralized network deployment
   - Integration with existing cooperative networks
   - User-friendly interfaces

## Contributing

The project is open for contributions. Key areas where help is needed:

- Implementing missing components
- Enhancing existing implementations
- Documentation and examples
- Testing and security reviews

## License

This project is licensed under the MIT OR Apache-2.0 license.
