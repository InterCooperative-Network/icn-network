# Network Architecture

This document describes the architecture and design considerations for the ICN Network crate.

## Overview

The ICN Network crate provides peer-to-peer networking functionality for the InterCooperative Network. It is built on top of the libp2p framework and offers a modular design that supports various network behaviors.

## Core Components

### Network Service

The `NetworkService` trait defines the interface for network operations:

```rust
#[async_trait]
pub trait NetworkService: Send + Sync + 'static {
    async fn start(&self) -> NetworkResult<()>;
    async fn stop(&self) -> NetworkResult<()>;
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()>;
    async fn send_to(&self, peer_id: &str, message: NetworkMessage) -> NetworkResult<()>;
    async fn connect(&self, addr: &Multiaddr) -> NetworkResult<()>;
    async fn disconnect(&self, peer_id: &str) -> NetworkResult<()>;
    async fn get_peer(&self, peer_id: &str) -> NetworkResult<Option<PeerInfo>>;
    async fn connected_peers(&self) -> NetworkResult<Vec<PeerInfo>>;
    async fn listen_addresses(&self) -> NetworkResult<Vec<Multiaddr>>;
    async fn register_message_handler(&self, topic: &str, handler: Arc<dyn MessageHandler>) -> NetworkResult<()>;
    fn local_peer_id(&self) -> String;
}
```

### P2P Network

The `P2pNetwork` implementation provides the actual peer-to-peer networking functionality. It:

1. Manages connections to peers
2. Handles network events
3. Routes messages between nodes
4. Integrates various libp2p protocols

### Message Handling

Messages are processed by handlers registered with the network:

1. Messages are received from the network
2. They're decoded and validated
3. Appropriate handlers are invoked based on the message type
4. Handlers can produce side effects or response messages

### Discovery

Peer discovery is achieved through multiple mechanisms:

1. **mDNS**: For local network discovery
2. **Kademlia DHT**: For decentralized discovery across the internet
3. **Bootstrap Peers**: Known peers for initial connection
4. **Persistent Peer Storage**: To remember previously connected peers

### Synchronization

State synchronization ensures all nodes have a consistent view of the network:

1. Nodes exchange state information
2. Missing items are requested and provided
3. Validation ensures correctness of the synchronized state

## Protocol Stack

The network uses the following libp2p protocols:

| Protocol | Purpose |
|----------|---------|
| TCP | Transport layer |
| Noise | Encryption |
| Yamux | Stream multiplexing |
| Identify | Peer metadata exchange |
| Ping | Connection liveness |
| Kademlia | Distributed Hash Table |
| mDNS | Local peer discovery |
| Gossipsub | Pub/sub messaging |
| Request/Response | Direct peer communication |

## Message Types

The network supports different message types for various functionalities:

| Message Type | Purpose |
|--------------|---------|
| IdentityAnnouncement | Announce a new identity |
| TransactionAnnouncement | Announce a new transaction |
| LedgerStateUpdate | Update the ledger state |
| ProposalAnnouncement | Announce a governance proposal |
| VoteAnnouncement | Announce a vote on a proposal |
| CustomMessage | Application-specific messages |

## Design Decisions

### Choice of libp2p

We chose libp2p because:

1. It's modular and provides the necessary protocols
2. It supports multiple transports (TCP, WebRTC, etc.)
3. It's actively maintained and has a strong community
4. It provides good abstractions for peer-to-peer networking

### Asynchronous API

The API is fully asynchronous using `async/await` and the Tokio runtime to:

1. Handle numerous simultaneous connections efficiently
2. Avoid blocking operations
3. Support high concurrency

### Gossipsub for Message Broadcasting

Gossipsub was chosen for broadcasting messages because:

1. It's efficient at propagating messages through the network
2. It provides message deduplication
3. It supports topic-based subscriptions
4. It scales well with network size

### Separation of Networking from Business Logic

The network layer is kept separate from application-specific logic:

1. The network layer focuses on message delivery and peer connections
2. Application-specific logic is implemented in message handlers
3. This separation allows for easier testing and modular development

### Error Handling

Comprehensive error handling is provided through:

1. Custom error types with descriptive messages
2. Error propagation using the `?` operator
3. Fallback mechanisms for recoverable errors

## Performance Considerations

### Connection Management

The network manages connections to optimize resource usage:

1. Idle connections are periodically pruned
2. Connection limits prevent resource exhaustion
3. Connection quality is monitored and poor connections are dropped

### Message Prioritization

Messages are prioritized based on their type:

1. Governance-related messages get higher priority
2. Large messages can be chunked to avoid blocking other messages
3. Critical messages have retry mechanisms

### Bandwidth Usage

Bandwidth is managed through:

1. Message compression when appropriate
2. Efficient protocol encodings
3. Rate limiting for peers to prevent DoS

## Security Considerations

### Message Authentication

All messages are authenticated:

1. Messages include sender identity information
2. Signatures verify message authenticity
3. Invalid messages are discarded

### Peer Authentication

Peers are authenticated when connecting:

1. Peer IDs are derived from public keys
2. The Noise protocol establishes secure connections
3. Peer reputations are tracked

### Denial of Service Protection

The network includes DoS protection mechanisms:

1. Rate limiting of messages from each peer
2. Resource limits per connection
3. Blacklisting of misbehaving peers

## Future Improvements

1. **WebRTC Transport**: Add WebRTC support for browser connectivity
2. **NAT Traversal**: Improve NAT traversal capabilities
3. **Circuit Relay**: Support relaying for nodes behind restrictive NATs
4. **Peer Reputation System**: Enhanced reputation tracking
5. **Network Metrics**: Comprehensive metrics collection
6. **Optimized Message Serialization**: For bandwidth efficiency 