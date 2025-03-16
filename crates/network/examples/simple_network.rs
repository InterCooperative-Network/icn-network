use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

use icn_core::storage::mock_storage::MockStorage;
use icn_network::{
    P2pNetwork, P2pConfig, MessageProcessor, NetworkMessage,
    TransactionAnnouncement, DefaultMessageHandler, PeerInfo,
    NetworkResult, DiscoveryManager, DiscoveryConfig,
};
use libp2p::{Multiaddr, PeerId};

#[tokio::main]
async fn main() -> Result<()> {
    // Set up logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting simple network example");
    
    // Create storage for both nodes
    let storage1 = Arc::new(MockStorage::new());
    let storage2 = Arc::new(MockStorage::new());
    
    // Create network configurations
    let mut config1 = P2pConfig::default();
    config1.listen_addresses = vec!["/ip6/::1/tcp/9001".parse()?];
    
    let mut config2 = P2pConfig::default();
    config2.listen_addresses = vec!["/ip6/::1/tcp/9002".parse()?];
    
    // Create the networks
    let network1 = Arc::new(P2pNetwork::new(storage1.clone(), config1).await?);
    let network2 = Arc::new(P2pNetwork::new(storage2.clone(), config2).await?);
    
    // Get node 1's address to use as a bootstrap for node 2
    network1.start().await?;
    sleep(Duration::from_millis(100)).await;
    
    let node1_peer_id = network1.local_peer_id().to_string();
    let node1_listen_addr = network1.listen_addresses().await?[0].clone();
    
    info!("Node 1 peer ID: {}", node1_peer_id);
    info!("Node 1 listening on: {}", node1_listen_addr);
    
    // Set up discovery for node 2 with node 1 as bootstrap
    let mut discovery_config = DiscoveryConfig::default();
    discovery_config.bootstrap_peers = vec![node1_listen_addr.clone()];
    
    // Create a message processor for node 1
    let message_processor1 = Arc::new(MessageProcessor::new());
    
    // Create a flag to check when a message is received
    let received_message = Arc::new(Mutex::new(false));
    let received_message_clone = received_message.clone();
    
    // Create a message handler
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "TransactionHandler".to_string(),
        move |message, peer| {
            info!("Node 1 received message from {}: {:?}", peer.peer_id, message);
            
            if let NetworkMessage::TransactionAnnouncement(tx) = message {
                info!("Transaction announcement: {}", tx.transaction_id);
                let mut received = received_message_clone.blocking_lock();
                *received = true;
            }
            
            Ok(())
        }
    ));
    
    // Register the handler
    message_processor1.register_handler("ledger.transaction", handler).await;
    
    // Start node 2
    network2.start().await?;
    
    // Connect node 2 to node 1
    let node1_addr: Multiaddr = format!("{}/p2p/{}", node1_listen_addr, node1_peer_id).parse()?;
    info!("Node 2 connecting to: {}", node1_addr);
    
    let peer_id = network2.connect(&node1_addr).await?;
    info!("Connected to peer: {}", peer_id);
    
    // Wait a moment for the connection to stabilize
    sleep(Duration::from_secs(1)).await;
    
    // Create a test message
    let tx_announce = TransactionAnnouncement {
        transaction_id: "tx123".to_string(),
        transaction_type: "transfer".to_string(),
        timestamp: 12345,
        sender: "alice".to_string(),
        data_hash: "abcdef123456".to_string(),
    };
    
    let message = NetworkMessage::TransactionAnnouncement(tx_announce);
    
    // Send the message from node 2 to node 1
    info!("Node 2 broadcasting message");
    network2.broadcast(message).await?;
    
    // Wait for the message to be received
    for _ in 0..10 {
        sleep(Duration::from_millis(500)).await;
        
        let received = *received_message.lock().await;
        if received {
            info!("Message successfully received!");
            break;
        }
    }
    
    // Clean up
    network1.stop().await?;
    network2.stop().await?;
    
    info!("Example completed successfully");
    Ok(())
} 