use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, MessageHandler, Message,
    PeerInfo, NetworkResult, NetworkMessage, ReputationChange, 
};
use icn_core::storage::MockStorage;
use libp2p::PeerId;
use tracing_subscriber::FmtSubscriber;
use tokio::time;
use tracing::{info, warn, error};
use async_trait::async_trait;

// Define a simple message handler that will track message behavior
struct BehaviorTracker {
    network: Arc<P2pNetwork>,
    node_name: String,
    inject_errors: bool,
}

#[async_trait]
impl MessageHandler for BehaviorTracker {
    async fn handle_message(&self, message: &NetworkMessage, peer_info: &PeerInfo) -> NetworkResult<()> {
        let peer_id = PeerId::from_bytes(peer_info.id.clone())
            .map_err(|_| icn_network::NetworkError::DecodingError)?;
            
        info!("[{}] Received message from {}", self.node_name, peer_id);
        
        // Simulate occasional errors based on configured behavior
        if self.inject_errors && rand::random::<f32>() < 0.3 {
            warn!("[{}] Simulating message handling error", self.node_name);
            
            // Record negative reputation change
            self.network.update_reputation(&peer_id, ReputationChange::MessageFailure).await?;
            
            return Err(icn_network::NetworkError::Other("Simulated error".to_string()));
        }
        
        // Record successful message handling
        self.network.update_reputation(&peer_id, ReputationChange::MessageSuccess).await?;
        
        // Occasionally verify messages (simulating complex validation)
        if rand::random::<f32>() < 0.7 {
            info!("[{}] Message validation succeeded", self.node_name);
            self.network.update_reputation(&peer_id, ReputationChange::VerifiedMessage).await?;
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
    
    info!("Starting reputation demo...");
    
    // Create storage instances
    let storage1 = Arc::new(MockStorage::new());
    let storage2 = Arc::new(MockStorage::new());
    let storage3 = Arc::new(MockStorage::new());
    
    // Create network configurations
    let mut config1 = P2pConfig::default();
    config1.listen_addresses = vec!["/ip6/::1/tcp/10001".parse()?];
    config1.enable_reputation = true;
    
    let mut config2 = P2pConfig::default();
    config2.listen_addresses = vec!["/ip6/::1/tcp/10002".parse()?];
    config2.enable_reputation = true;
    
    let mut config3 = P2pConfig::default();
    config3.listen_addresses = vec!["/ip6/::1/tcp/10003".parse()?];
    config3.enable_reputation = true;
    
    // Create network instances
    let network1 = Arc::new(P2pNetwork::new(storage1, config1).await?);
    let network2 = Arc::new(P2pNetwork::new(storage2, config2).await?);
    let network3 = Arc::new(P2pNetwork::new(storage3, config3).await?);
    
    // Register message handlers
    let tracker1 = Arc::new(BehaviorTracker {
        network: network1.clone(),
        node_name: "Node1".to_string(),
        inject_errors: false,
    });
    
    let tracker2 = Arc::new(BehaviorTracker {
        network: network2.clone(),
        node_name: "Node2".to_string(),
        inject_errors: true, // This node will occasionally fail to handle messages
    });
    
    let tracker3 = Arc::new(BehaviorTracker {
        network: network3.clone(),
        node_name: "Node3".to_string(),
        inject_errors: false,
    });
    
    // Register handlers for all message types (just using transaction type for demo)
    network1.register_handler("ledger.transaction", tracker1).await?;
    network2.register_handler("ledger.transaction", tracker2).await?;
    network3.register_handler("ledger.transaction", tracker3).await?;
    
    // Start all networks
    network1.start().await?;
    network2.start().await?;
    network3.start().await?;
    
    // Connect networks
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    let peer_id3 = network3.local_peer_id()?;
    
    info!("Node 1 peer ID: {}", peer_id1);
    info!("Node 2 peer ID: {}", peer_id2);
    info!("Node 3 peer ID: {}", peer_id3);
    
    // Connect node 1 to both 2 and 3
    network1.connect(&format!("/ip6/::1/tcp/10002/p2p/{}", peer_id2)).await?;
    network1.connect(&format!("/ip6/::1/tcp/10003/p2p/{}", peer_id3)).await?;
    
    // Wait for connections to establish
    time::sleep(Duration::from_secs(2)).await;
    
    // Demo 1: Node 1 sends messages to Node 2 (which has errors) - reputation should decrease
    info!("==== DEMO 1: Node 1 sends messages to Node 2 (error-prone) ====");
    for i in 0..10 {
        let message = NetworkMessage {
            message_type: "ledger.transaction".to_string(),
            content: format!("Transaction {} from Node 1", i).into_bytes(),
        };
        
        // Send directly to node 2
        network1.send_message(&peer_id2, message).await?;
        
        // Small delay between messages
        time::sleep(Duration::from_millis(200)).await;
    }
    
    // Check reputation after first batch
    let node2_rep = network1.reputation_manager().unwrap()
        .get_reputation(&peer_id2).await;
    
    match node2_rep {
        Some(rep) => info!("Node 2 reputation after first batch: {}", rep.score()),
        None => info!("No reputation data for Node 2 yet"),
    }
    
    // Demo 2: Node 1 sends messages to Node 3 (no errors) - reputation should increase
    info!("==== DEMO 2: Node 1 sends messages to Node 3 (reliable) ====");
    for i in 0..10 {
        let message = NetworkMessage {
            message_type: "ledger.transaction".to_string(),
            content: format!("Transaction {} from Node 1", i).into_bytes(),
        };
        
        // Send directly to node 3
        network1.send_message(&peer_id3, message).await?;
        
        // Small delay between messages
        time::sleep(Duration::from_millis(200)).await;
    }
    
    // Check reputations
    let node2_rep = network1.reputation_manager().unwrap()
        .get_reputation(&peer_id2).await;
    
    let node3_rep = network1.reputation_manager().unwrap()
        .get_reputation(&peer_id3).await;
    
    match node2_rep {
        Some(rep) => info!("Node 2 final reputation: {}", rep.score()),
        None => info!("No reputation data for Node 2"),
    }
    
    match node3_rep {
        Some(rep) => info!("Node 3 final reputation: {}", rep.score()),
        None => info!("No reputation data for Node 3"),
    }
    
    // Demo 3: Banning a peer
    info!("==== DEMO 3: Ban and unban demonstration ====");
    
    // Check if node 2 is already banned
    let is_banned = network1.reputation_manager().unwrap()
        .is_banned(&peer_id2).await;
    
    if is_banned {
        info!("Node 2 is already banned due to poor reputation");
    } else {
        info!("Manually banning Node 2");
        network1.ban_peer(&peer_id2).await?;
        
        // Verify ban status
        let is_banned = network1.reputation_manager().unwrap()
            .is_banned(&peer_id2).await;
        
        info!("Node 2 banned status: {}", is_banned);
    }
    
    // Try to connect to banned peer (should fail or be ignored)
    let result = network1.connect(&format!("/ip6/::1/tcp/10002/p2p/{}", peer_id2)).await;
    match result {
        Ok(_) => info!("Connected to banned peer - connection was allowed but peer is still banned"),
        Err(e) => info!("Failed to connect to banned peer as expected: {}", e),
    }
    
    // Wait a moment
    time::sleep(Duration::from_secs(1)).await;
    
    // Now unban the peer
    info!("Unbanning Node 2");
    network1.unban_peer(&peer_id2).await?;
    
    // Verify ban status
    let is_banned = network1.reputation_manager().unwrap()
        .is_banned(&peer_id2).await;
    
    info!("Node 2 banned status after unban: {}", is_banned);
    
    // Try to connect to unbanned peer
    network1.connect(&format!("/ip6/::1/tcp/10002/p2p/{}", peer_id2)).await?;
    info!("Successfully connected to unbanned peer");
    
    // Demo 4: Reputation decay
    info!("==== DEMO 4: Reputation decay demonstration ====");
    info!("Waiting for reputation decay to occur...");
    
    // Record initial reputation
    let node3_initial_rep = network1.reputation_manager().unwrap()
        .get_reputation(&peer_id3).await
        .map(|r| r.score())
        .unwrap_or(0);
    
    info!("Node 3 initial reputation: {}", node3_initial_rep);
    
    // Wait for decay to occur (would be faster with a test-specific configuration)
    time::sleep(Duration::from_secs(60)).await;
    
    // Check reputation after decay
    let node3_after_decay = network1.reputation_manager().unwrap()
        .get_reputation(&peer_id3).await
        .map(|r| r.score())
        .unwrap_or(0);
    
    info!("Node 3 reputation after decay: {}", node3_after_decay);
    if node3_after_decay < node3_initial_rep {
        info!("Reputation successfully decayed over time");
    } else {
        info!("No decay observed in this short period (may need longer or different config for demo)");
    }
    
    // Clean shutdown
    info!("Shutting down networks...");
    network1.stop().await?;
    network2.stop().await?;
    network3.stop().await?;
    
    info!("Reputation demo completed successfully!");
    Ok(())
} 