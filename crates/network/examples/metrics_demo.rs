use std::sync::Arc;
use std::time::Duration;

use icn_core::storage::mock_storage::MockStorage;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, NetworkMessage,
    DefaultMessageHandler, TransactionAnnouncement, Timer,
};
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,icn_network=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Create storage
    let storage = Arc::new(MockStorage::new());
    
    // Configure the first network node with metrics enabled
    let mut config1 = P2pConfig::default();
    config1.listen_addresses = vec!["/ip4/127.0.0.1/tcp/10001".parse()?];
    config1.enable_metrics = true;
    config1.metrics_address = Some("127.0.0.1:9091".to_string());
    
    // Configure the second network node without metrics
    let mut config2 = P2pConfig::default();
    config2.listen_addresses = vec!["/ip4/127.0.0.1/tcp/10002".parse()?];
    
    println!("Creating network nodes...");
    
    // Create two network nodes
    let network1 = Arc::new(P2pNetwork::new(storage.clone(), config1).await?);
    let network2 = Arc::new(P2pNetwork::new(storage.clone(), config2).await?);
    
    println!("Starting network nodes...");
    
    // Start both networks
    network1.start().await?;
    network2.start().await?;
    
    // Wait for networks to initialize
    sleep(Duration::from_millis(100)).await;
    
    // Get node 1's address for connecting
    let node1_peer_id = network1.local_peer_id();
    let node1_addr = network1.listen_addresses().await?[0].clone();
    let node1_full_addr = format!("{}/p2p/{}", node1_addr, node1_peer_id);
    
    println!("Node 1 address: {}", node1_full_addr);
    
    // Create a message handler for node 1
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "MetricsDemo".to_string(),
        move |message, peer_id| {
            println!("Received message from {}: {:?}", peer_id, message);
            Ok(())
        }
    ));
    
    // Register the handler
    network1.register_message_handler("demo.transaction", handler).await?;
    
    println!("Connecting node 2 to node 1...");
    
    // Connect node 2 to node 1
    network2.connect(&node1_full_addr.parse()?).await?;
    
    println!("Connected! Starting message exchange...");
    println!("Metrics available at http://127.0.0.1:9091");
    
    // Send messages in a loop to generate metrics
    for i in 1..=15 {
        // Create a test message with varying size
        let data_size = i * 100; // Increase size with each iteration
        let tx_announce = TransactionAnnouncement {
            transaction_id: format!("tx-{}", i),
            transaction_type: "metrics-demo".to_string(),
            timestamp: 12345,
            sender: "demo-node".to_string(),
            data_hash: "0".repeat(data_size),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx_announce);
        
        println!("Sending message #{} (size: {} bytes)...", i, data_size);
        
        // Broadcast the message
        network2.broadcast(message).await?;
        
        // Wait between messages
        sleep(Duration::from_secs(2)).await;
    }
    
    // Add some connection metrics
    println!("Testing connections and disconnections...");
    
    for i in 1..=5 {
        println!("Disconnect and reconnect cycle {}", i);
        
        // Disconnect
        network2.disconnect(&node1_peer_id).await?;
        sleep(Duration::from_secs(1)).await;
        
        // Reconnect
        network2.connect(&node1_full_addr.parse()?).await?;
        sleep(Duration::from_secs(1)).await;
    }
    
    println!("Demo completed. Metrics server still running at http://127.0.0.1:9091");
    println!("Press Ctrl+C to exit");
    
    // Keep the application running
    tokio::signal::ctrl_c().await?;
    
    // Stop networks
    network1.stop().await?;
    network2.stop().await?;
    
    println!("Networks stopped");
    
    Ok(())
} 