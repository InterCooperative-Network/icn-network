# ICN Network

The network communication layer for the InterCooperative Network (ICN). This crate provides a peer-to-peer networking solution using libp2p, enabling nodes in the network to discover each other, exchange messages, and synchronize state.

## Features

- **Peer-to-peer networking** - Establish connections with other nodes in the network
- **Peer discovery** - Find other nodes using various discovery mechanisms
- **Messaging** - Send and receive different types of messages
- **State synchronization** - Keep the network state in sync across nodes
- **Metrics and monitoring** - Collect and expose performance metrics
- **Peer Reputation System** - Track and manage peer reliability and behavior
- **Priority Message Processing** - Process messages based on reputation and message type priority
- **Circuit Relay** - Enable NAT traversal for nodes behind firewalls

## Components

### P2P Network

The core P2P networking functionality is implemented in the `P2pNetwork` struct, which provides:

- Starting and stopping the network service
- Broadcasting messages to the entire network
- Sending messages to specific peers
- Connecting to and disconnecting from peers
- Retrieving peer information

### Discovery

The discovery module provides mechanisms for finding other nodes in the network:

- Local network discovery using mDNS
- Distributed Hash Table (DHT) based discovery using Kademlia
- Bootstrap peer list for initial connections

### Messaging

The messaging module handles the exchange of various message types:

- Identity announcements
- Transaction announcements
- Ledger state updates
- Governance proposal announcements
- Vote announcements
- Custom messages

### Synchronization

The synchronization module ensures that all nodes have a consistent view of the network state:

- Ledger state synchronization
- Identity state synchronization
- Governance state synchronization

### Metrics

The metrics module provides comprehensive monitoring of network performance:

- Connection metrics (peers connected, connections established, disconnects)
- Message metrics (messages received/sent by type, message size, processing time)
- Discovery metrics (peers discovered, discovery methods)
- Resource usage (memory, CPU)
- Error tracking

## Usage

### Basic Example

```rust
use std::sync::Arc;
use icn_core::storage::mock_storage::MockStorage;
use icn_network::{P2pNetwork, P2pConfig, NetworkService};

async fn main() -> anyhow::Result<()> {
    // Create a storage backend
    let storage = Arc::new(MockStorage::new());
    
    // Configure the network
    let mut config = P2pConfig::default();
    config.listen_addresses = vec!["/ip4/0.0.0.0/tcp/8000".parse()?];
    
    // Create and start the network
    let network = P2pNetwork::new(storage, config).await?;
    network.start().await?;
    
    // Connect to a peer
    let peer_addr = "/ip4/127.0.0.1/tcp/8001/p2p/QmPeerID".parse()?;
    network.connect(&peer_addr).await?;
    
    // Broadcast a message
    let message = /* create a message */;
    network.broadcast(message).await?;
    
    // Stop the network when done
    network.stop().await?;
    
    Ok(())
}
```

## Testing and Demos

### Using the Makefile

This crate includes a Makefile to simplify common tasks:

```bash
# Build the crate
make build

# Run unit tests
make unit-tests

# Run integration tests
make integration-tests

# Run all tests
make test

# Run the metrics demo
make metrics

# Run the reputation demo
make reputation

# Run the priority messaging demo
make priority

# Run the circuit relay demo
make relay

# Run the integrated demo with all features
make integrated

# Run the interactive demo script
make demo
```

### Automated Testing

To run comprehensive tests of all features:

```bash
./scripts/test_all_features.sh
```

This script automatically tests:
1. Metrics and monitoring system
2. Peer reputation management
3. Priority-based message processing
4. Circuit relay for NAT traversal
5. The integrated demo combining all features

### Interactive Demos

For a guided exploration of the network features:

```bash
./scripts/run_demos.sh
```

This interactive script allows you to:
- Run individual feature demos
- Run the integrated demo with various configurations
- Try out the circuit relay demo with different node types

### Enabling Metrics

To enable metrics collection and exposure:

```rust
// Configure the network with metrics
let mut config = P2pConfig::default();
config.listen_addresses = vec!["/ip4/0.0.0.0/tcp/8000".parse()?];
config.enable_metrics = true;
config.metrics_address = Some("127.0.0.1:9090".to_string());

// Create the network with metrics enabled
let network = P2pNetwork::new(storage, config).await?;
```

This will start a Prometheus-compatible metrics server at the specified address, which you can scrape with Prometheus or query directly in your browser.

Available metrics include:
- `network_peers_connected` - Number of connected peers
- `network_messages_received` - Number of messages received by type
- `network_messages_sent` - Number of messages sent by type
- `network_message_processing_time` - Time to process messages
- `network_peers_discovered` - Number of peers discovered
- And many more

## Peer Reputation System

The network includes a reputation system that tracks peer behavior and makes decisions about which peers to trust, prioritize, or avoid.

### Key Features

- **Behavior Tracking**: Monitors peer actions (connection stability, message handling, response times)
- **Reputation Scoring**: Assigns and updates reputation scores based on behavior
- **Automatic Banning**: Can automatically ban peers that fall below reputation thresholds
- **Decay Over Time**: Reputation scores gradually decay toward neutral over time
- **Metric Integration**: Exposes reputation data through the metrics system
- **Persistence**: Can save and load reputation data between restarts

### Enabling the Reputation System

```rust
let mut config = P2pConfig::default();
config.enable_reputation = true;

// Optional custom configuration
let reputation_config = ReputationConfig {
    ban_threshold: -50,               // Score below which peers are automatically banned
    decay_factor: 0.05,               // Rate at which scores decay toward 0
    decay_interval: Duration::from_secs(3600), // Time between decay operations
    good_threshold: 25,               // Score above which peers are considered "good"
    fast_response_threshold: 100,     // Response time (ms) considered "fast"
    slow_response_threshold: 1000,    // Response time (ms) considered "slow"
    // ...other options
};
config.reputation_config = Some(reputation_config);
```

### Using the Reputation System

```rust
use icn_network::{ReputationChange, NetworkService};
use libp2p::PeerId;

async fn manage_peer_reputation(network: &P2pNetwork, peer_id: &PeerId) -> Result<(), Box<dyn std::error::Error>> {
    // Get the reputation manager
    let reputation = network.reputation_manager().unwrap();
    
    // Record reputation changes
    reputation.record_change(peer_id, ReputationChange::MessageSuccess).await?;
    reputation.record_change(peer_id, ReputationChange::VerifiedMessage).await?;
    
    // Check if a peer is banned
    let is_banned = reputation.is_banned(peer_id).await;
    println!("Peer {} banned status: {}", peer_id, is_banned);
    
    // Explicitly ban a peer
    network.ban_peer(peer_id).await?;
    
    // Unban a peer
    network.unban_peer(peer_id).await?;
    
    // Get a peer's reputation
    let rep = reputation.get_reputation(peer_id).await;
    if let Some(rep) = rep {
        println!("Peer reputation score: {}", rep.score());
    }
    
    Ok(())
}
```

## Priority Message Processing

The network includes a priority-based message processing system that allows messages from trusted peers and high-priority message types to be processed before others. This is especially useful during high load situations or when dealing with critical transactions.

### Key Features

- **Message Prioritization**: Processes messages based on calculated priority rather than just order of receipt
- **Multiple Priority Modes**: Offers several prioritization strategies:
  - **Type-based**: Prioritize by message type (e.g., votes before transactions)
  - **Reputation-based**: Prioritize messages from peers with higher reputation
  - **Combined**: Use both type and sender reputation
  - **FIFO**: Traditional first-in, first-out processing (default)
- **Backpressure Handling**: Managed queue size with configurable drop strategies
- **Performance Metrics**: Detailed metrics for monitoring queue sizes and processing times

### Enabling Priority Processing

```rust
let mut config = P2pConfig::default();
config.enable_message_prioritization = true;

// Optional custom priority configuration
let priority_config = PriorityConfig {
    mode: PriorityMode::TypeAndReputation,
    high_priority_message_types: vec!["consensus.vote".to_string(), "ledger.transaction".to_string()],
    high_priority_reputation: 20,  // Reputation threshold for high priority
    max_queue_size: 10000,         // Maximum message queue size
    // ...other options
};
config.priority_config = Some(priority_config);
```

## Circuit Relay for NAT Traversal

The network includes a circuit relay protocol that allows nodes behind NATs or firewalls to connect to other nodes through publicly accessible relay nodes. This significantly improves connectivity in real-world deployments.

### Key Features

- **NAT Traversal**: Connect nodes that would otherwise be unreachable due to NAT or firewalls
- **Relay Server**: Run a node as a relay server to facilitate connections between peers
- **Relay Client**: Connect through relay servers to reach otherwise inaccessible peers
- **Smart Connection**: Automatically attempt direct connection before falling back to relay
- **Relay Prioritization**: Choose the best relay based on connection success rates
- **Connection Monitoring**: Track and report statistics on relayed connections

### Enabling Circuit Relay

```rust
let mut config = P2pConfig::default();
config.enable_circuit_relay = true;

// Optional custom relay configuration
let mut relay_config = CircuitRelayConfig::default();
relay_config.enable_relay_server = true;  // Act as a relay server (optional)
relay_config.enable_relay_client = true;  // Connect through relays (default)
relay_config.known_relay_servers = vec![
    "/ip4/public-relay.example.com/tcp/4001/p2p/QmRelayId".parse()?
];
relay_config.max_inbound_relay_connections = 20;  // Maximum inbound relay connections
relay_config.ttl = Duration::from_secs(3600);     // Time to keep relay connections alive

config.circuit_relay_config = Some(relay_config);
```

### Using Circuit Relay

```rust
use icn_network::{P2pNetwork, NetworkService};
use libp2p::PeerId;

async fn connect_to_peer(network: &P2pNetwork, peer_id: &PeerId) -> anyhow::Result<()> {
    // Smart connect will try direct connection first, then fall back to relay
    network.smart_connect(peer_id).await?;
    
    // Check if the connection is relayed
    let is_relayed = network.is_relay_connection(peer_id).await;
    println!("Connection to {} is relayed: {}", peer_id, is_relayed);
    
    // If relayed, get the relay peer ID
    if is_relayed {
        if let Some(relay_id) = network.get_relay_for_connection(peer_id).await {
            println!("Using relay: {}", relay_id);
        }
    }
    
    Ok(())
}
```

## License

This project is dual-licensed under:
- MIT License
- Apache License, Version 2.0