use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, MessageHandler, 
    PeerInfo, NetworkResult, NetworkMessage, ReputationChange, messaging,
    TransactionAnnouncement
};
use icn_core::storage::MockStorage;
use libp2p::PeerId;
use tracing_subscriber::FmtSubscriber;
use tokio::time;
use tracing::{info, warn, error, debug};
use async_trait::async_trait;

// Test handler to log received messages and their priorities
struct PriorityTestHandler {
    node_name: String,
    // Keep track of received messages for testing
    received_high_priority: std::sync::atomic::AtomicUsize,
    received_low_priority: std::sync::atomic::AtomicUsize,
}

impl PriorityTestHandler {
    fn new(node_name: &str) -> Self {
        Self {
            node_name: node_name.to_string(),
            received_high_priority: std::sync::atomic::AtomicUsize::new(0),
            received_low_priority: std::sync::atomic::AtomicUsize::new(0),
        }
    }
    
    fn get_stats(&self) -> (usize, usize) {
        (
            self.received_high_priority.load(std::sync::atomic::Ordering::Relaxed),
            self.received_low_priority.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

#[async_trait]
impl MessageHandler for PriorityTestHandler {
    fn id(&self) -> usize {
        0
    }
    
    fn name(&self) -> &str {
        &self.node_name
    }
    
    async fn handle_message(&self, message: &NetworkMessage, peer: &PeerInfo) -> NetworkResult<()> {
        if let Ok(peer_id) = libp2p::PeerId::from_bytes(&peer.id) {
            // Determine if this is a high or low priority message from content
            let content = match message {
                NetworkMessage::TransactionAnnouncement(tx) => &tx.transaction_id,
                _ => "",
            };
            
            if content.contains("high_priority") {
                info!("[{}] Received HIGH PRIORITY message from {}: {:?}", 
                      self.node_name, peer_id, message);
                self.received_high_priority.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            } else {
                debug!("[{}] Received LOW PRIORITY message from {}: {:?}", 
                       self.node_name, peer_id, message);
                self.received_low_priority.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }
        
        // Simulate processing time - sleep longer for low priority messages
        // to demonstrate the prioritization effect
        if message.to_string().contains("low_priority") {
            time::sleep(Duration::from_millis(50)).await;
        } else {
            time::sleep(Duration::from_millis(10)).await;
        }
        
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(tracing::Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting priority-based message processing demo...");
    
    // Create storage instances
    let storage1 = Arc::new(MockStorage::new());
    let storage2 = Arc::new(MockStorage::new());
    
    // Setup network configs
    let mut config1 = P2pConfig::default();
    config1.listen_addresses = vec!["/ip4/127.0.0.1/tcp/10101".parse()?];
    config1.enable_reputation = true;
    config1.enable_message_prioritization = true;
    
    // Create custom priority config
    let mut priority_config = messaging::PriorityConfig::default();
    priority_config.mode = messaging::PriorityMode::TypeAndReputation;
    priority_config.high_priority_reputation = 10; // Lower threshold for demo
    config1.priority_config = Some(priority_config);
    
    // Node 2 doesn't need prioritization
    let mut config2 = P2pConfig::default();
    config2.listen_addresses = vec!["/ip4/127.0.0.1/tcp/10102".parse()?];
    
    // Create and start networks
    let network1 = Arc::new(P2pNetwork::new(storage1, config1).await?);
    let network2 = Arc::new(P2pNetwork::new(storage2, config2).await?);
    
    // Create test handlers
    let handler1 = Arc::new(PriorityTestHandler::new("Node1"));
    
    // Register handlers
    network1.register_message_handler("ledger.transaction", handler1.clone()).await?;
    
    // Start networks
    network1.start().await?;
    network2.start().await?;
    
    // Get peer IDs
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    
    info!("Node 1 peer ID: {}", peer_id1);
    info!("Node 2 peer ID: {}", peer_id2);
    
    // Connect Node 2 to Node 1
    info!("Connecting node 2 to node 1...");
    network2.connect(&format!("/ip4/127.0.0.1/tcp/10101/p2p/{}", peer_id1)).await?;
    
    // Wait for connection to establish
    time::sleep(Duration::from_secs(1)).await;
    
    // First, build up some reputation for node 2 from node 1's perspective
    info!("Building up reputation for Node 2...");
    let reputation = network1.reputation_manager().unwrap();
    for _ in 0..5 {
        reputation.record_change(&peer_id2, ReputationChange::MessageSuccess).await?;
    }
    
    // Get queue stats before sending messages
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Initial queue stats - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Send 50 low priority messages and 10 high priority messages interleaved
    info!("Sending mixed priority messages...");
    
    // First batch: sending only low priority messages
    info!("Phase 1: Sending 20 low priority messages");
    for i in 0..20 {
        let tx = TransactionAnnouncement {
            transaction_id: format!("low_priority_tx_{}", i),
            transaction_type: "transfer".to_string(),
            timestamp: i as u64,
            sender: "test".to_string(),
            data_hash: "abcdef".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
        
        // Small delay between messages
        time::sleep(Duration::from_millis(10)).await;
    }
    
    // Let some messages be processed
    time::sleep(Duration::from_millis(200)).await;
    
    // Get queue stats after first batch
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Queue stats after low priority batch - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Phase 2: Sending mixed priority messages
    info!("Phase 2: Sending mixed priority messages (10 high, 30 low)");
    
    // Create and send both low and high priority messages
    for i in 0..40 {
        let is_high_priority = i % 4 == 0; // Every 4th message is high priority
        
        let tx = TransactionAnnouncement {
            transaction_id: if is_high_priority { 
                format!("high_priority_tx_{}", i / 4) 
            } else { 
                format!("low_priority_tx_{}", 20 + i) 
            },
            transaction_type: "transfer".to_string(),
            timestamp: (i + 20) as u64,
            sender: "test".to_string(),
            data_hash: "abcdef".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
        
        // Small delay between messages
        time::sleep(Duration::from_millis(10)).await;
    }
    
    // Let the messages be processed
    time::sleep(Duration::from_millis(500)).await;
    
    // Get queue stats after all messages
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Queue stats after all messages - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Wait for message processing to complete
    time::sleep(Duration::from_secs(2)).await;
    
    // Output message processing statistics
    let (high, low) = handler1.get_stats();
    info!("Message processing statistics:");
    info!("High priority messages received: {}", high);
    info!("Low priority messages received: {}", low);
    info!("Total messages received: {}", high + low);
    
    // Demonstrate how higher priority messages get processed first
    info!("Sending a burst of 30 more messages with mixed priorities...");
    
    // Reset counters for this test
    handler1.received_high_priority.store(0, std::sync::atomic::Ordering::Relaxed);
    handler1.received_low_priority.store(0, std::sync::atomic::Ordering::Relaxed);
    
    // Send burst of mixed messages
    for i in 0..30 {
        let is_high_priority = i < 5; // First 5 are high priority
        
        let tx = TransactionAnnouncement {
            transaction_id: if is_high_priority { 
                format!("high_priority_burst_{}", i) 
            } else { 
                format!("low_priority_burst_{}", i) 
            },
            transaction_type: "transfer".to_string(),
            timestamp: (i + 100) as u64,
            sender: "test".to_string(),
            data_hash: "abcdef".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
    }
    
    // Wait briefly for some processing to start
    time::sleep(Duration::from_millis(200)).await;
    
    // Check intermediate stats - high priority messages should be processed first
    let (high, low) = handler1.get_stats();
    info!("Intermediate processing statistics:");
    info!("High priority messages received: {} (should be close to 5)", high);
    info!("Low priority messages received: {}", low);
    
    // Wait for all messages to be processed
    time::sleep(Duration::from_secs(3)).await;
    
    // Final statistics
    let (high, low) = handler1.get_stats();
    info!("Final processing statistics:");
    info!("High priority messages received: {}", high);
    info!("Low priority messages received: {}", low);
    info!("Total messages received: {}", high + low);
    
    // Clean up
    network1.stop().await?;
    network2.stop().await?;
    
    info!("Priority-based message processing demo completed!");
    Ok(())
} 