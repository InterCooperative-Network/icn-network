# ICN Network

The network layer for the InterCooperative Network (ICN) system.

## Overview

This crate provides the networking capabilities for ICN, allowing nodes to communicate in a peer-to-peer fashion. It handles node discovery, secure messaging, and state synchronization between nodes.

## Features

- **Peer-to-peer networking** using libp2p
- **Decentralized node discovery** using mDNS, Kademlia DHT, and bootstrap nodes
- **Secure messaging** with identity verification
- **State synchronization** for ledger and governance data
- **Message handlers** for processing different message types
- **Configurable network behavior**

## Components

The network crate consists of several key components:

### P2P Network

The `p2p` module provides the core peer-to-peer networking functionality, including:

- Connection management
- Message routing
- Protocol handling
- Network event processing

### Discovery

The `discovery` module handles node discovery mechanisms, such as:

- Bootstrap peers
- mDNS for local network discovery
- Kademlia DHT for decentralized discovery
- Persistent peer storage

### Messaging

The `messaging` module handles message processing, including:

- Message encoding and decoding
- Message routing to appropriate handlers
- Message validation

### Synchronization

The `sync` module handles state synchronization between nodes, including:

- Ledger state synchronization
- Governance state synchronization
- Identity synchronization

## Usage

### Creating a Network

```rust
use std::sync::Arc;
use icn_core::storage::Storage;
use icn_network::{P2pNetwork, P2pConfig};

async fn create_network(storage: Arc<dyn Storage>) -> Result<Arc<P2pNetwork>, Box<dyn std::error::Error>> {
    // Create network configuration
    let mut config = P2pConfig::default();
    config.listen_addresses = vec!["/ip4/0.0.0.0/tcp/9000".parse()?];
    
    // Create the network
    let network = Arc::new(P2pNetwork::new(storage, config).await?);
    
    // Start the network
    network.start().await?;
    
    Ok(network)
}
```

### Handling Messages

```rust
use std::sync::Arc;
use icn_network::{MessageProcessor, NetworkMessage, DefaultMessageHandler, PeerInfo, NetworkResult};

async fn setup_message_handling(message_processor: Arc<MessageProcessor>) {
    // Create a message handler
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "TransactionHandler".to_string(),
        move |message, peer| {
            println!("Received message from {}: {:?}", peer.peer_id, message);
            Ok(())
        }
    ));
    
    // Register the handler for transaction messages
    message_processor.register_handler("ledger.transaction", handler).await;
}
```

### Sending Messages

```rust
use icn_network::{NetworkMessage, TransactionAnnouncement};

async fn send_message(network: &P2pNetwork) -> NetworkResult<()> {
    // Create a transaction announcement
    let tx_announce = TransactionAnnouncement {
        transaction_id: "tx123".to_string(),
        transaction_type: "transfer".to_string(),
        timestamp: 12345,
        sender: "alice".to_string(),
        data_hash: "abcdef123456".to_string(),
    };
    
    // Create the network message
    let message = NetworkMessage::TransactionAnnouncement(tx_announce);
    
    // Broadcast the message to all connected peers
    network.broadcast(message).await?;
    
    Ok(())
}
```

### State Synchronization

```rust
use std::sync::Arc;
use icn_network::{Synchronizer, SyncConfig};

async fn setup_synchronization(
    network: Arc<dyn NetworkService>,
    storage: Arc<dyn Storage>
) -> NetworkResult<Arc<Synchronizer>> {
    // Create sync configuration
    let config = SyncConfig::default();
    
    // Create the synchronizer
    let synchronizer = Arc::new(Synchronizer::new(
        storage,
        network,
        config,
    ));
    
    // Start synchronization
    synchronizer.start().await?;
    
    Ok(synchronizer)
}
```

## Examples

The crate includes examples that demonstrate the usage of the network layer:

- `simple_network.rs`: A simple example of two nodes communicating with each other.

Run the examples using:

```bash
cargo run --example simple_network
```

## Configuration

The network behavior can be configured using the following configuration structs:

- `P2pConfig`: Configuration for the P2P network
- `DiscoveryConfig`: Configuration for peer discovery
- `SyncConfig`: Configuration for state synchronization

## Dependencies

- `libp2p`: Core peer-to-peer networking library
- `tokio`: Asynchronous runtime
- `serde`: Serialization and deserialization
- `tracing`: Logging and tracing

## License

This crate is licensed under MIT OR Apache-2.0, the same as the rest of the ICN project. 