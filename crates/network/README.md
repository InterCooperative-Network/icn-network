# ICN Network

The network communication layer for the InterCooperative Network (ICN). This crate provides a peer-to-peer networking solution using libp2p, enabling nodes in the network to discover each other, exchange messages, and synchronize state.

## Features

- **Peer-to-peer networking** - Establish connections with other nodes in the network
- **Peer discovery** - Find other nodes using various discovery mechanisms
- **Messaging** - Send and receive different types of messages
- **State synchronization** - Keep the network state in sync across nodes
- **Metrics and monitoring** - Collect and expose performance metrics
- **Peer Reputation System** - Track and manage peer reliability and behavior

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

## CLI Tool

The crate includes a command-line interface for testing and demonstration purposes. You can run it using:

```bash
# Start a listening node
cargo run --example network_cli -- -p 8000 listen

# Connect to another node
cargo run --example network_cli -- -p 8001 connect -p /ip4/127.0.0.1/tcp/8000/p2p/<PEER_ID>

# Broadcast messages
cargo run --example network_cli -- -p 8002 broadcast -p /ip4/127.0.0.1/tcp/8000/p2p/<PEER_ID> -i 2 -c 10
```

## Metrics Demo

Try out the metrics functionality with the provided demo:

```bash
# Run the metrics demo
cargo run --example metrics_demo

# Visit http://127.0.0.1:9091 in your browser to see the metrics
```

## Benchmarks

Performance benchmarks are included to measure the network's efficiency:

```bash
# Run all benchmarks
cargo bench

# Run a specific benchmark
cargo bench -- network_broadcast
```

## Testing

The crate includes comprehensive unit tests for all components:

```bash
# Run all tests
cargo test

# Run tests with logging
RUST_LOG=debug cargo test -- --nocapture
```

## License

Licensed under either of:

- Apache License, Version 2.0
- MIT License

at your option.

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

### Reputation Change Types

The system tracks multiple types of reputation changes:

- **ConnectionEstablished** (+10): Successful connection
- **ConnectionLost** (-5): Connection dropped unexpectedly
- **MessageSuccess** (+5): Successful message exchange
- **MessageFailure** (-10): Failed message exchange
- **InvalidMessage** (-20): Malformed or invalid message
- **VerifiedMessage** (+15): Successfully validated message
- **DiscoveryHelp** (+5): Helped with peer discovery
- **SlowResponse** (-2): Slow to respond
- **FastResponse** (+1): Quick to respond
- **ExplicitBan** (-100): Manually banned by administrator

### Reputation Demo

Run the reputation system demo to see it in action:

```
cargo run --example reputation_demo
```

## CLI Tool

A command-line tool is included for testing and debugging network functionality:

```
USAGE:
    icn-net [OPTIONS] [SUBCOMMAND]

OPTIONS:
    -h, --help       Print help information
    -v, --verbose    Verbose output

SUBCOMMANDS:
    help           Print help information
    listen         Start a listening node
    connect        Connect to another node
    broadcast      Broadcast a message
    metrics        Start a node with metrics enabled
    reputation     Run the reputation system demo
```

### Listen Example:

```
icn-net listen --port 10001
```

### Connect Example:

```
icn-net connect --target /ip4/127.0.0.1/tcp/10001/p2p/QmYyQSo1c1Ym7orWxLYvCrM2EmxFTANf8wXmmE7DWjhx5N
```

### Broadcast Example:

```
icn-net broadcast --type "ledger.transaction" --content "Hello World"
```

### Run Metrics Demo:

```
icn-net metrics
```

### Run Reputation Demo:

```
icn-net reputation
``` 