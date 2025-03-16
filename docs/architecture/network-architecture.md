# ICN Network Architecture

This document provides a detailed technical overview of the ICN Network architecture, focusing on the core components and their interactions.

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    ICN Node Architecture                         │
├─────────────┬────────────────┬────────────────┬─────────────────┤
│             │                │                │                 │
│  Identity   │   Networking   │    Economic    │   Governance    │
│    Layer    │     Layer      │     Layer      │     Layer       │
│             │                │                │                 │
├─────────────┼────────────────┼────────────────┼─────────────────┤
│             │                │                │                 │
│  DID System │ libp2p Network │  Mutual Credit │  Voting System  │
│             │                │                │                 │
├─────────────┼────────────────┼────────────────┼─────────────────┤
│             │                │                │                 │
│ Credentials │   WireGuard    │  Transactions  │    Proposals    │
│             │                │                │                 │
├─────────────┼────────────────┼────────────────┼─────────────────┤
│             │                │                │                 │
│  Key Mgmt   │ Service Disc.  │ Resource Alloc │ Decision Making │
│             │                │                │                 │
└─────────────┴────────────────┴────────────────┴─────────────────┘
```

The ICN system is built around modular, interacting components that collectively provide a decentralized infrastructure for cooperative networks.

## 1. Identity Layer

The Identity Layer implements decentralized identity management using the W3C DID specification.

### 1.1 DID Implementation

#### Core Components

- **DID Manager**: Creates and manages DIDs following the format `did:icn:<coop-id>:<user-id>`
- **DID Resolver**: Resolves DIDs to DID Documents
- **Verification Module**: Handles cryptographic proof verification

#### DID Document Structure

```json
{
  "id": "did:icn:coopA:userX",
  "controller": ["did:icn:coopA:admin"],
  "verificationMethod": [{
    "id": "did:icn:coopA:userX#keys-1",
    "type": "Ed25519VerificationKey2020",
    "controller": "did:icn:coopA:userX",
    "publicKeyMultibase": "z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK"
  }],
  "authentication": ["did:icn:coopA:userX#keys-1"],
  "service": [{
    "id": "did:icn:coopA:userX#wireguard",
    "type": "WireGuardEndpoint",
    "serviceEndpoint": "wg://fd00:abcd:1234::1"
  }]
}
```

#### Storage Mechanisms

1. **DHT Storage**
   - Primary storage method for fast resolution
   - Uses libp2p Kademlia DHT
   - Records expire and need renewal

2. **Blockchain Storage**
   - Persistent, authoritative storage
   - Used for verification and as fallback
   - Enables revocation and audit trails

### 1.2 Authentication Flow

```
┌──────────┐          ┌──────────┐         ┌──────────┐
│          │          │          │         │          │
│  Client  │          │  Relay   │         │  Target  │
│          │          │  Server  │         │  Service │
└────┬─────┘          └────┬─────┘         └────┬─────┘
     │                     │                    │
     │  1. AuthRequest     │                    │
     │────────────────────>│                    │
     │                     │                    │
     │                     │  2. Forward Auth   │
     │                     │───────────────────>│
     │                     │                    │
     │                     │  3. Challenge      │
     │                     │<───────────────────│
     │  4. Challenge       │                    │
     │<────────────────────│                    │
     │                     │                    │
     │  5. Signed Response │                    │
     │────────────────────>│                    │
     │                     │  6. Forward        │
     │                     │───────────────────>│
     │                     │                    │
     │                     │  7. Verify Sig     │
     │                     │  8. Check Auth     │
     │                     │                    │
     │                     │  9. Auth Token     │
     │                     │<───────────────────│
     │ 10. Auth Token      │                    │
     │<────────────────────│                    │
     │                     │                    │
     │ 11. Access with Token                    │
     │─────────────────────────────────────────>│
     │                     │                    │
     │ 12. Service Response                     │
     │<─────────────────────────────────────────│
     │                     │                    │
```

1. Authentication begins with an AuthRequest containing the user's DID
2. The target service issues a cryptographic challenge
3. The client signs the challenge with their private key
4. The service verifies the signature against the DID Document's public key
5. Upon successful verification, the service issues a JWT or similar token

### 1.3 Implementation Details

```rust
// DID Manager implementation
pub struct DidManager {
    // The DID resolver
    resolver: Arc<DidResolver>,
    // Storage for DIDs
    storage: Arc<dyn Storage>,
    // Private keys stored by DID
    private_keys: HashMap<String, PrivateKey>,
}

impl DidManager {
    // Create a new DID
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(String, DidDocument)> {
        // Generate DID identifier
        let id = self.generate_did_identifier(options.coop_id.clone(), options.user_id.clone());
        
        // Create verification method with public key
        let verification_method = VerificationMethod {
            id: format!("{}#keys-1", id),
            controller: id.clone(),
            type_: options.key_type.clone(),
            public_key_multibase: encode_public_key(&options.keypair.public()),
        };
        
        // Build the DID document
        let document = DidDocument {
            id: id.clone(),
            controller: vec![],
            verification_method: vec![verification_method.clone()],
            authentication: vec![VerificationMethodReference::Reference(format!("{}#keys-1", id))],
            // ... other document fields
        };
        
        // Store in DHT and/or blockchain
        self.storage.store_did_document(&id, &document).await?;
        
        // Store private key
        self.private_keys.insert(id.clone(), options.keypair.private());
        
        Ok((id, document))
    }
}
```

## 2. Networking Layer

The Networking Layer provides secure communication between nodes using a combination of libp2p and WireGuard.

### 2.1 libp2p Network

#### Core Components

- **P2P Network**: Built on libp2p for peer discovery and communication
- **Transport Manager**: Handles multiple transport protocols
- **Circuit Relay**: Enables NAT traversal for nodes behind firewalls

#### Protocol Stack

| Layer | Protocol |
|-------|----------|
| Transport | TCP, QUIC, WebSocket, WebRTC |
| Security | Noise, TLS |
| Multiplexing | Yamux, mplex |
| Peer Discovery | mDNS, Kademlia DHT, Bootstrap Nodes |
| Pubsub | GossipSub |
| Services | Identify, Ping, Circuit Relay |

#### Implementation Details

```rust
// P2P Network implementation
pub struct P2pNetwork {
    // Storage for network data
    storage: Arc<dyn Storage>,
    // libp2p key pair
    key_pair: Keypair,
    // Local peer ID
    local_peer_id: PeerId,
    // Network configuration
    config: P2pConfig,
    // Known peers
    peers: Arc<RwLock<HashMap<PeerId, PeerInfo>>>,
    // Reputation manager
    reputation: Option<Arc<ReputationManager>>,
    // Message processor
    message_processor: Option<Arc<MessageProcessor>>,
    // Circuit relay manager
    circuit_relay: Option<Arc<CircuitRelayManager>>,
    // Swarm instance
    swarm: Arc<Mutex<Option<swarm::Swarm<P2pBehaviour>>>>,
}

impl P2pNetwork {
    // Create a new P2P network
    pub async fn new(storage: Arc<dyn Storage>, config: P2pConfig) -> NetworkResult<Self> {
        // Initialize components
        let key_pair = Self::load_or_create_keypair(storage.clone()).await?;
        let local_peer_id = PeerId::from(key_pair.public());
        
        // Create the libp2p transport
        let transport = Self::build_transport(&key_pair, &config).await?;
        
        // Create the swarm with behaviors
        let swarm = Self::create_swarm(
            &key_pair, 
            transport, 
            &config
        ).await?;
        
        // Initialize the network
        let network = Self { /* initialize fields */ };
        
        Ok(network)
    }
}
```

### 2.2 WireGuard Integration

#### Core Components

- **WireGuard Manager**: Configures and manages WireGuard interfaces
- **Key Exchange**: Uses libp2p DHT to exchange WireGuard public keys
- **IP Allocator**: Assigns unique IPv6 addresses to nodes

#### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         ICN Node                                 │
│                                                                 │
│  ┌─────────────┐      ┌─────────────┐      ┌─────────────┐      │
│  │             │      │             │      │             │      │
│  │  libp2p     │<────>│  WireGuard  │<────>│  Local      │      │
│  │  Network    │      │  Interface  │      │  Services   │      │
│  │             │      │             │      │             │      │
│  └─────────────┘      └─────────────┘      └─────────────┘      │
│        │                                                        │
│        │                                                        │
│        V                                                        │
│  ┌─────────────┐                                                │
│  │             │                                                │
│  │  DHT        │                                                │
│  │  (Key Store)│                                                │
│  │             │                                                │
│  └─────────────┘                                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### WireGuard Configuration Flow

1. **Key Generation**: Each node generates a WireGuard keypair
2. **DID Association**: WireGuard public key is added to DID Document
3. **Key Discovery**: Nodes discover peers' public keys via DHT
4. **Dynamic Configuration**: WireGuard interface is configured dynamically
5. **Tunnel Establishment**: Secure tunnel is established between peers

#### Implementation Details

```rust
pub struct WireguardManager {
    storage: Arc<dyn Storage>,
    dht: Arc<DhtService>,
    config: WireguardConfig,
    device: WgDevice,
    interface_name: String,
    keypair: WgKeypair,
    ipv6_prefix: Ipv6Net,
    peer_configs: Arc<RwLock<HashMap<String, WireguardPeerConfig>>>,
}

impl WireguardManager {
    // Configure WireGuard for a peer
    async fn configure_peer_tunnel(&self, peer_did: &str) -> Result<()> {
        // Resolve the peer's DID to get their WireGuard public key
        let did_doc = self.dht.resolve_did(peer_did).await?;
        
        // Find the WireGuard service endpoint in the DID document
        let wg_service = did_doc.find_service("WireGuardEndpoint")?;
        
        // Extract public key from DID document
        let public_key = wg_service.get_public_key()?;
        
        // Get peer's IPv6 address
        let ipv6_address = self.resolve_did_to_ipv6(peer_did).await?;
        
        // Configure WireGuard interface
        self.device.add_peer(
            public_key,
            None, // We don't need an endpoint for the overlay network
            &[ipv6_address], // Allow traffic to peer's IPv6 address
            25, // Keep-alive interval
        )?;
        
        Ok(())
    }
}
```

### 2.3 Service Discovery

#### Core Components

- **DHT Service Registry**: Stores and retrieves service information
- **Name Resolution**: Maps human-readable names to network addresses
- **Service Advertisement**: Allows nodes to announce their services

#### Service Record Structure

```json
{
  "serviceId": "database-coopA",
  "type": "database",
  "provider": "did:icn:coopA:node1",
  "endpoints": [
    {
      "transport": "wg",
      "address": "fd00:abcd:1234::1",
      "port": 5432
    },
    {
      "transport": "libp2p",
      "address": "/ip4/192.168.1.1/tcp/9000/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N"
    }
  ],
  "metadata": {
    "version": "12.4",
    "compatibility": ["12.x", "11.x"],
    "protocol": "postgresql"
  },
  "accessControl": {
    "federation": "fed:icn:cooperative-alliance",
    "roles": ["member", "partner"]
  }
}
```

#### Implementation Details

```rust
pub struct ServiceRegistry {
    dht: Arc<DhtService>,
    blockchain: Option<Arc<BlockchainClient>>,
    local_services: Arc<RwLock<HashMap<String, ServiceRecord>>>,
}

impl ServiceRegistry {
    // Register a service in the DHT and optionally in the blockchain
    pub async fn register_service(&self, service: ServiceRecord) -> Result<()> {
        // Validate the service record
        self.validate_service_record(&service)?;
        
        // Store in local cache
        self.local_services.write().await.insert(service.service_id.clone(), service.clone());
        
        // Store in DHT for fast lookup
        let key = format!("service:{}", service.service_id);
        let serialized = serde_json::to_vec(&service)?;
        self.dht.put(key.into_bytes(), serialized).await?;
        
        // If blockchain storage is available, store there for persistence and auditing
        if let Some(blockchain) = &self.blockchain {
            blockchain.store_service_record(&service).await?;
        }
        
        Ok(())
    }
    
    // Lookup a service by ID
    pub async fn lookup_service(&self, service_id: &str) -> Result<ServiceRecord> {
        // Try local cache first
        if let Some(service) = self.local_services.read().await.get(service_id) {
            return Ok(service.clone());
        }
        
        // Try DHT lookup
        let key = format!("service:{}", service_id);
        match self.dht.get(key.into_bytes()).await {
            Ok(value) => {
                let service: ServiceRecord = serde_json::from_slice(&value)?;
                return Ok(service);
            }
            Err(_) => {
                // Fall back to blockchain if available
                if let Some(blockchain) = &self.blockchain {
                    return blockchain.get_service_record(service_id).await;
                }
                
                return Err(Error::not_found(format!("Service not found: {}", service_id)));
            }
        }
    }
    
    // Resolve a human-readable name to a service record
    pub async fn resolve_name(&self, name: &str) -> Result<ServiceRecord> {
        // Split the name into parts (service.coop.icn)
        let parts: Vec<&str> = name.split('.').collect();
        if parts.len() != 3 || parts[2] != "icn" {
            return Err(Error::invalid_input("Invalid name format"));
        }
        
        // Construct a search key
        let service_name = parts[0];
        let coop_id = parts[1];
        
        // Look up in DHT
        let key = format!("name:{}:{}", coop_id, service_name);
        match self.dht.get(key.into_bytes()).await {
            Ok(value) => {
                let service_id = String::from_utf8(value)?;
                return self.lookup_service(&service_id).await;
            }
            Err(_) => {
                // Fall back to blockchain
                if let Some(blockchain) = &self.blockchain {
                    return blockchain.resolve_name(name).await;
                }
                
                return Err(Error::not_found(format!("Name not resolved: {}", name)));
            }
        }
    }
}
```

## 3. Integration Points

### 3.1 DID and WireGuard Integration

The DID system and WireGuard are integrated through the following mechanisms:

1. **DID Document Service Endpoints**: WireGuard public keys are stored in DID Documents
2. **DHT-based Key Exchange**: WireGuard keys are discovered via DID resolution
3. **Authentication-triggered Tunnel Setup**: When a node authenticates, WireGuard tunnels are established

### 3.2 Service Discovery and libp2p Integration

Service discovery leverages libp2p's DHT capabilities:

1. **DHT Records**: Services are registered as key-value pairs in the libp2p Kademlia DHT
2. **Record Validation**: Records are validated against the DID of the publisher
3. **Multi-transport Support**: Services advertise both libp2p and WireGuard endpoints

### 3.3 Authentication and Service Access

The authentication system integrates with service access control:

1. **Verifiable Credentials**: Services check credentials during access attempts
2. **Role-based Access**: Federation-level roles determine access permissions
3. **Federation Boundaries**: Services can restrict access to specific federations

## 4. Deployment Architecture

### 4.1 Standalone Node

A standalone ICN node consists of:

```
┌─────────────────────────────────────────────────────────┐
│                    ICN Node                             │
│                                                         │
│  ┌─────────────┐    ┌─────────────┐   ┌─────────────┐   │
│  │             │    │             │   │             │   │
│  │   DID       │    │  libp2p     │   │  Storage    │   │
│  │   Manager   │    │  Network    │   │  System     │   │
│  │             │    │             │   │             │   │
│  └─────────────┘    └─────────────┘   └─────────────┘   │
│         │                 │                 │           │
│         └─────────────────┼─────────────────┘           │
│                           │                             │
│                    ┌──────▼──────┐                      │
│                    │             │                      │
│                    │  WireGuard  │                      │
│                    │  Interface  │                      │
│                    │             │                      │
│                    └─────────────┘                      │
│                           │                             │
└───────────────────────────┼─────────────────────────────┘
                            │
                      ┌─────▼──────┐
                      │            │
                      │  External  │
                      │  Network   │
                      │            │
                      └────────────┘
```

### 4.2 Cooperative Deployment

A typical cooperative deployment includes multiple nodes with different roles:

```
┌─────────────────────────────────────┐
│          Cooperative Network         │
│                                     │
│  ┌─────────┐    ┌─────────┐         │
│  │         │    │         │         │
│  │ Primary │    │ Backup  │         │
│  │ Node    │    │ Node    │         │
│  │         │    │         │         │
│  └────┬────┘    └────┬────┘         │
│       │              │              │
│       └──────────────┘              │
│              │                      │
│     ┌────────┴───────┐              │
│     │                │              │
│  ┌──▼───┐        ┌───▼──┐           │
│  │      │        │      │           │
│  │ Peer │        │ Peer │           │
│  │ Node │        │ Node │           │
│  │      │        │      │           │
│  └──────┘        └──────┘           │
│                                     │
└─────────────────────────────────────┘
```

### 4.3 Federation Architecture

Multiple cooperatives form a federation:

```
┌───────────────────────────────────────────────────────────────┐
│                         Federation                            │
│                                                               │
│  ┌─────────────────┐   ┌─────────────────┐   ┌──────────────┐ │
│  │   Cooperative A │   │   Cooperative B │   │ Cooperative C│ │
│  │                 │   │                 │   │              │ │
│  │  ┌────┐  ┌────┐ │   │  ┌────┐  ┌────┐ │   │  ┌────┐      │ │
│  │  │Node│  │Node│ │   │  │Node│  │Node│ │   │  │Node│      │ │
│  │  └────┘  └────┘ │   │  └────┘  └────┘ │   │  └────┘      │ │
│  │                 │   │                 │   │              │ │
│  └────────┬────────┘   └────────┬────────┘   └──────┬───────┘ │
│           │                     │                   │         │
│           └─────────────────────┼───────────────────┘         │
│                                 │                             │
│                        ┌────────▼─────────┐                   │
│                        │                  │                   │
│                        │  Federation      │                   │
│                        │  Registry        │                   │
│                        │                  │                   │
│                        └──────────────────┘                   │
│                                                               │
└───────────────────────────────────────────────────────────────┘
```

## 5. Security Considerations

### 5.1 Trust Model

The ICN employs a hybrid trust model:

1. **Zero-Trust Authentication**: All access requires cryptographic proof
2. **Federation-based Trust**: Federations establish trust boundaries
3. **Reputation-based Trust**: Node behavior affects trust over time

### 5.2 Attack Vectors and Mitigations

| Attack Vector | Mitigation |
|---------------|------------|
| Sybil Attacks | Federation validation, proof-of-personhood |
| Eclipse Attacks | Multiple discovery methods, trusted bootstrap nodes |
| MitM Attacks | End-to-end encryption, key verification |
| DoS Attacks | Rate limiting, reputation system, resource quotas |
| Key Compromise | Key rotation, revocation in blockchain |

### 5.3 Privacy Considerations

The ICN prioritizes user privacy:

1. **Minimal Data Exchange**: Only necessary data is shared
2. **Consent-based Sharing**: Users control credential disclosure
3. **Confidential Connections**: All traffic is encrypted
4. **Federation Boundaries**: Data remains within federation unless explicitly shared

## 6. Scalability and Performance

### 6.1 Scalability Mechanisms

The ICN is designed to scale through:

1. **Hierarchical Structure**: Federations group cooperatives to limit global state
2. **DHT Optimization**: Specialized DHT for frequently accessed records
3. **Caching**: Multi-level caching of resolution results
4. **Lazy Loading**: On-demand loading of remote information

### 6.2 Performance Optimizations

Key performance optimizations include:

1. **Connection Pooling**: Reuse connections for multiple requests
2. **Message Prioritization**: Critical messages are processed first
3. **Parallel Resolution**: Multiple resolution paths tried in parallel
4. **Incremental Sync**: Only sync changes, not full state

## 7. Implementation Roadmap

### 7.1 Phase 1: Identity & Authentication

- Implement DID manager and resolver
- Add DHT-based identity storage and retrieval
- Build authentication verification system

### 7.2 Phase 2: WireGuard Integration

- Create WireGuard configuration manager
- Add key storage/retrieval via DHT
- Implement dynamic tunnel configuration

### 7.3 Phase 3: Name Resolution & Service Discovery

- Add hostname resolution via DHT
- Implement blockchain fallback
- Build service advertising system

### 7.4 Phase 4: Federation & Cross-Coop Access

- Create federation registry
- Implement cross-coop authentication
- Add dynamic permission enforcement

## 8. Further Reading

- [DID W3C Specification](https://www.w3.org/TR/did-core/)
- [libp2p Documentation](https://docs.libp2p.io/)
- [WireGuard Protocol](https://www.wireguard.com/protocol/)
- [Kademlia DHT Paper](https://pdos.csail.mit.edu/~petar/papers/maymounkov-kademlia-lncs.pdf) 