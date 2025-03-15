# ICN Network

The network communication layer for the InterCooperative Network (ICN). This crate provides a peer-to-peer networking solution using libp2p, enabling nodes in the network to discover each other, exchange messages, and synchronize state.

## Features

- **Peer-to-peer networking** - Establish connections with other nodes in the network
- **Peer discovery** - Find other nodes using various discovery mechanisms
- **Messaging** - Send and receive different types of messages
- **State synchronization** - Keep the network state in sync across nodes
- **Metrics and monitoring** - Collect and expose performance metrics

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