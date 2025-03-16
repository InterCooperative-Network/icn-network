# ICN Network

A comprehensive peer-to-peer networking solution for distributed applications, with a focus on performance, security, and reliability.

## Features

- **Peer-to-Peer Networking**: Fully decentralized communication using libp2p
- **Decentralized Identity (DID)**: W3C-compliant DID implementation for authentication and identity management
- **Circuit Relay**: NAT traversal for connecting nodes behind firewalls
- **WireGuard Federation**: Secure overlay networking with dynamic peer configuration
- **Peer Discovery**: Multiple discovery mechanisms (mDNS, Kademlia DHT, bootstrap peers)
- **DHT-based Name Resolution**: Decentralized alternative to DNS for service discovery
- **Zero-Trust Authentication**: Cryptographic peer verification without centralized authorities
- **Reputation System**: Intelligent tracking of peer behavior with automatic banning of misbehaving nodes
- **Priority Message Processing**: Smart message queue that prioritizes important messages and high-reputation peers
- **Metrics and Monitoring**: Comprehensive metrics collection for network performance tracking

## Architecture Overview

The ICN Network employs a layered architecture:

1. **Identity Layer**: Handles DID-based identity and authentication
2. **Transport Layer**: Manages secure connections via libp2p and WireGuard
3. **Discovery Layer**: Provides peer discovery and service resolution
4. **Messaging Layer**: Handles message exchange and prioritization
5. **Application Layer**: Domain-specific logic built on top of the network

## Authentication & Identity

### Decentralized Identity (DID)

The system implements the W3C DID specification with an `icn` method:

```
did:icn:coopA:userX
```

- **DID Resolution**: DIDs can be resolved via DHT or blockchain fallback
- **Verification Methods**: Support for multiple key types (Ed25519, secp256k1)
- **Credentials**: Verifiable credentials for authorization and attribute verification

### Zero-Trust Authentication

Users authenticate using:
- **Cryptographic Signatures**: Ed25519/secp256k1 public key cryptography
- **WebAuthn Support**: For hardware security key integration
- **WireGuard Peer Authentication**: Network-level authentication

## WireGuard Integration

### Dynamic WireGuard Peering

- **DHT-based Key Distribution**: Peer WireGuard keys are stored and retrieved via libp2p DHT
- **Auto-Configuration**: Tunnels are dynamically configured based on authentication state
- **End-to-End Encryption**: All traffic between nodes is secured

### IPv6 Overlay Network

- **Dynamic IP Assignment**: Nodes receive IPv6 addresses from a private range
- **DID-to-IP Mapping**: DIDs are mapped to IPv6 addresses via the DHT
- **Cross-Coop Routing**: Seamless routing between cooperative networks

## Name Resolution & Service Discovery

### DHT-based Name Resolution

- **Decentralized DNS Alternative**: Resolves `name.coop.icn` to the appropriate node
- **Service Advertisement**: Nodes advertise their services in the DHT
- **Blockchain Fallback**: Authoritative name verification via blockchain

### Multi-Protocol Support

- **Transport Negotiation**: Nodes negotiate optimal transport protocols (IPv6, QUIC, WebRTC)
- **Protocol Discovery**: Services advertise supported protocols in DHT records
- **Fallback Mechanisms**: Graceful degradation when preferred protocols are unavailable

## Project Structure

- `crates/network/` - Main network crate
  - `src/` - Source code
    - `p2p.rs` - Core libp2p implementation
    - `discovery.rs` - Peer discovery mechanisms
    - `circuit_relay.rs` - NAT traversal
    - `identity.rs` - DID implementation
    - `wireguard.rs` - WireGuard integration
    - `messaging.rs` - Message handling
    - `reputation.rs` - Peer reputation system
    - `metrics.rs` - Performance monitoring
  - `examples/` - Example applications
  - `scripts/` - Utility scripts
  - `tests/` - Integration tests

## Getting Started

### Prerequisites

- Rust 1.70 or later
- Cargo
- WireGuard (for overlay network functionality)
- (For running tests) curl and jq

### Building the Project

```bash
cargo build
```

### Running Examples

```bash
# Run a specific example
cargo run --example did_auth_demo

# Start a basic ICN node
cargo run --example icn_node -- --listen /ip4/0.0.0.0/tcp/9000
```

## Testing

### Running Unit Tests

```bash
cargo test
```

### Running Integration Tests

```bash
./crates/network/scripts/test_all_features.sh
```

## Demo Applications

The project includes several demos:

- **DID Auth Demo**: Demonstrates DID-based authentication
- **WireGuard Demo**: Shows dynamic WireGuard tunnel configuration
- **DHT Resolution Demo**: Demonstrates peer and service discovery
- **Federation Demo**: Shows cross-cooperative communication
- **Integrated Demo**: Combines all features into a single application

Run any demo using:

```bash
cargo run --example <demo_name>
```

## Documentation

For more detailed documentation, see:

- [Network Architecture](crates/network/docs/ARCHITECTURE.md) - Details on the network design
- [DID Implementation](docs/identity/did-implementation.md) - Details on the DID system
- [WireGuard Integration](docs/networking/wireguard-integration.md) - WireGuard overlay network
- API Documentation (generate with `cargo doc --open`)

## Development Roadmap

1. **Phase 1: Identity & Authentication**
   - Implement DID manager and resolver
   - Add DHT-based identity storage and retrieval
   - Build authentication verification system

2. **Phase 2: WireGuard Integration**
   - Create WireGuard configuration manager
   - Add key storage/retrieval via DHT
   - Implement dynamic tunnel configuration

3. **Phase 3: Name Resolution & Service Discovery**
   - Add hostname resolution via DHT
   - Implement blockchain fallback
   - Build service advertising system

4. **Phase 4: Federation & Cross-Coop Access**
   - Create federation registry
   - Implement cross-coop authentication
   - Add dynamic permission enforcement

## License

This project is dual-licensed under:
- MIT License
- Apache License, Version 2.0
