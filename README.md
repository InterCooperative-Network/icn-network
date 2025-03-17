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
- **End-to-End Encrypted Storage**: Versioned storage system with federation-based encryption and fine-grained access control
- **Governance-Controlled Storage**: Democratic policy management for storage with access control and quotas
- **Identity-Integrated Storage**: DID-based authentication and access control for secure data management
- **Credential-Based Storage**: Verifiable credential-based access control with attribute verification

## Architecture Overview

The ICN Network employs a layered architecture:

1. **Identity Layer**: Handles DID-based identity and authentication
2. **Transport Layer**: Manages secure connections via libp2p and WireGuard
3. **Discovery Layer**: Provides peer discovery and service resolution
4. **Messaging Layer**: Handles message exchange and prioritization
5. **Governance Layer**: Enables democratic decision-making through proposals and voting
6. **Storage Layer**: Provides secure, versioned, governed storage with DID and credential integration
7. **Application Layer**: Domain-specific logic built on top of the network

## Authentication & Identity

### Decentralized Identity (DID)

The system implements the W3C DID specification with an `icn` method:

```
did:icn:coopA:userX
```

- **DID Resolution**: DIDs can be resolved via DHT or blockchain fallback
- **Verification Methods**: Support for multiple key types (Ed25519, secp256k1)
- **Credentials**: Verifiable credentials for authorization and attribute verification
- **DID-Based Storage**: Authentication and access control using DIDs for storage operations
- **Verifiable Credentials**: W3C-compliant verifiable credentials for attribute-based access control

### Zero-Trust Authentication

Users authenticate using:
- **Cryptographic Signatures**: Ed25519/secp256k1 public key cryptography
- **WebAuthn Support**: For hardware security key integration
- **WireGuard Peer Authentication**: Network-level authentication
- **Credential Verification**: Attribute-based authentication using verifiable credentials

## WireGuard Integration

### Dynamic WireGuard Peering

- **DHT-based Key Distribution**: Peer WireGuard keys are stored and retrieved via libp2p DHT
- **Auto-Configuration**: Tunnels are dynamically configured based on authentication state
- **End-to-End Encryption**: All traffic between nodes is secured

### IPv6 Overlay Network

- **Dynamic IP Assignment**: Nodes receive IPv6 addresses from a private range
- **DID-to-IP Mapping**: DIDs are mapped to IPv6 addresses via the DHT
- **Cross-Coop Routing**: Seamless routing between cooperative networks

## Democratic Governance

The ICN Network includes a democratic governance system that enables federation members to collectively make decisions:

- **Proposal System**: Members can create proposals for policy changes, resource allocations, etc.
- **Deliberation Period**: Time for discussion and refinement of proposals
- **Voting Mechanism**: Secure, transparent voting with configurable quorum and approval thresholds
- **Execution**: Automatic execution of approved proposals
- **Policy Management**: Governance of network rules, access controls, and resource allocation

## Name Resolution & Service Discovery

### DHT-based Name Resolution

- **Decentralized DNS Alternative**: Resolves `name.coop.icn` to the appropriate node
- **Service Advertisement**: Nodes advertise their services in the DHT
- **Blockchain Fallback**: Authoritative name verification via blockchain

### Multi-Protocol Support

- **Transport Negotiation**: Nodes negotiate optimal transport protocols (IPv6, QUIC, WebRTC)
- **Protocol Discovery**: Services advertise supported protocols in DHT records
- **Fallback Mechanisms**: Graceful degradation when preferred protocols are unavailable

## Secure Storage System

### Multi-Federation Encrypted Storage

The ICN Network includes a secure, versioned storage system that supports both symmetric and asymmetric encryption:

- **Federation-based Storage Isolation**: Separate storage environments with independent encryption
- **Automatic Versioning**: Full version history with cryptographic verification
- **End-to-End Encryption**: Multiple encryption options including ChaCha20Poly1305 and AES-GCM
- **Recipient-specific Encryption**: Encrypt files for specific recipients using X25519 keypairs
- **Key Management**: Secure key storage, federation key sharing, and key rotation
- **Password-derived Keys**: Support for password-based encryption using Argon2 key derivation

### Governance-Controlled Storage Policies

The storage system is integrated with the governance system, allowing democratic control over:

- **Access Control Policies**: Who can access which files, with pattern-based rules
- **Storage Quotas**: Federation-wide and per-member storage limits
- **Encryption Requirements**: Mandating encryption for specific file types
- **Retention Policies**: Rules for version history management
- **Replication Policies**: How data is replicated across storage nodes

### Identity-Integrated Storage

The Identity-Integrated Storage System combines governance-controlled policies with decentralized identity for authentication and access control:

- **DID-Based Authentication**: Users authenticate using DIDs and cryptographic signatures
- **Identity-to-Member Mapping**: DIDs are mapped to federation member IDs for policy enforcement
- **Key Rotation Support**: Users can update their DID documents and keys while maintaining access
- **Challenge-Response Authentication**: Secure authentication through challenge signing
- **DID-Based Access Control**: Fine-grained control over who can access specific resources

### Credential-Based Storage

The Credential-Based Storage System extends identity-integrated storage with attribute-based access control using verifiable credentials:

- **Attribute-Based Access Control**: Grant access based on verified attributes like role, department, or clearance
- **Credential Verification**: Cryptographically verify credentials before authorizing access
- **Expiration & Revocation Checking**: Automatically enforce credential freshness and validity
- **Fine-Grained Rule Matching**: Match file patterns against credential types and attribute values
- **Trust Framework Integration**: Federation-governed decisions on trusted credential issuers
- **Rule Persistency**: Stable enforcement of access rules with save/load functionality

### Usage via CLI

```bash
# Initialize storage with encryption enabled
icn-cli storage init --path ./data --encrypted

# Generate federation encryption key
icn-cli storage generate-key --output ./federation.key

# Store encrypted file
icn-cli storage put --file document.pdf --encrypted --federation finance

# Retrieve encrypted file
icn-cli storage get --key document.pdf --output ./retrieved.pdf --federation finance

# Generate asymmetric key pair for recipient-specific encryption
icn-cli storage generate-key-pair --output-dir ./my_keys

# Encrypt file for specific recipients
icn-cli storage encrypt-for --input sensitive.doc --output sensitive.enc --recipients "user1_pub.key,user2_pub.key"

# Decrypt file with your private key
icn-cli storage decrypt-with --input sensitive.enc --output decrypted.doc --private-key ./my_keys/private.key

# Store file with governance permission checks
icn-cli governed-storage store-file --file document.pdf --member alice@example.org

# Propose a new storage policy
icn-cli governed-storage propose-policy --proposer alice@example.org --title "New Access Controls" --policy-type access-control --content-file policy.json

# Register a DID for storage access
icn-cli identity-storage register-did --did "did:icn:alice" --document alice_did.json

# Store a file with DID authentication
icn-cli identity-storage store-file --did "did:icn:alice" --challenge "timestamp=1621500000" --signature "alice_signature" --file secret.txt --encrypted

# Register a verifiable credential
icn-cli credential-storage register-credential --credential hr_credential.json --federation my-federation

# Create a credential-based access rule
icn-cli credential-storage create-access-rule --did "did:icn:alice" --challenge "timestamp=1621500000" --signature "alice_signature" --pattern "hr_*" --credential-types "DepartmentCredential" --attributes '{"department": "HR"}' --permissions "read,write" --federation my-federation

# Store a file with credential authentication
icn-cli credential-storage store-file --did "did:icn:alice" --challenge "timestamp=1621500010" --signature "alice_signature" --credential-id "credential:1" --file document.txt --key "document.txt" --encrypted --federation my-federation
```

### Storage Implementation

The storage system implements:

- **Content Integrity Verification**: SHA-256 hashing with authenticated encryption
- **Hybrid Encryption**: Public key encryption with symmetric content keys for efficiency
- **Key Isolation**: Separate key stores with memory protection
- **Secure Key Derivation**: Argon2id for password-based encryption
- **Multiple Recipient Support**: Encrypting for multiple recipients without re-encrypting content
- **Democratic Policy Enforcement**: Governance-based access control and quota management
- **DID Authentication**: Cryptographic verification of DID control for storage operations
- **Credential Verification**: Validate verifiable credentials for attribute-based authorization
- **Attribute Matching**: Match credential attributes against access rules for fine-grained control

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

# Run the storage encryption demo
./examples/storage_encryption_demo.sh

# Run the governance-controlled storage demo
./examples/governed_storage_demo.sh

# Run the identity-integrated storage demo
./examples/identity_storage_demo.sh
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
- **Storage Encryption Demo**: Demonstrates the secure storage capabilities
- **Governed Storage Demo**: Shows governance-controlled storage policies
- **Identity Storage Demo**: Demonstrates DID-based storage authentication and access control
- **Credential Storage Demo**: Shows attribute-based access control using verifiable credentials
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
- [Secure Storage](docs/storage/secure-storage.md) - Encrypted federation-based storage
- [Governance-Controlled Storage](docs/storage/governance-controlled-storage.md) - Democratic storage management
- [Identity-Integrated Storage](docs/storage/identity-integrated-storage.md) - DID-based storage authentication
- API Documentation (generate with `cargo doc --open`)

## Development Roadmap

1. **Phase 1: Identity & Authentication**
   - Implement DID manager and resolver
   - Add DHT-based identity storage and retrieval

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

5. **Phase 5: Secure Storage System**
   - Implement versioned storage with encryption
   - Add federation-based isolation
   - Enable secure key management
   - Support recipient-specific encryption
   - Add secure key sharing mechanisms

6. **Phase 6: Governance & Democracy**
   - Implement proposal and voting system
   - Create policy enforcement framework
   - Integrate governance with storage
   - Add distributed execution of approved proposals

7. **Phase 7: Distributed Applications**
   - Create application hosting framework
   - Implement service discovery for applications
   - Add secure inter-application communication
   - Develop application governance mechanisms

## License

This project is dual-licensed under:
- MIT License
- Apache License, Version 2.0
