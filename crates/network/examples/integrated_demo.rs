use std::sync::Arc;
use std::time::Duration;
use futures::StreamExt;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, MessageHandler, 
    PeerInfo, NetworkResult, NetworkMessage, ReputationChange, messaging,
    TransactionAnnouncement, ReputationConfig
};
use icn_core::storage::MockStorage;
use libp2p::PeerId;
use tracing_subscriber::FmtSubscriber;
use tokio::time;
use tracing::{info, warn, error, debug};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use clap::{Parser, Subcommand};

/// Integrated demo for ICN Network features: metrics, reputation, and priority messaging
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Enable verbose logging
    #[clap(short, long)]
    verbose: bool,

    /// Metrics server address
    #[clap(short, long, default_value = "127.0.0.1:9090")]
    metrics_address: String,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run the complete integrated demo
    All,
    
    /// Run only the metrics demo
    Metrics,
    
    /// Run only the reputation demo
    Reputation,
    
    /// Run only the priority messaging demo
    Priority,
}

// Integrated message handler that tracks metrics for all message types
struct IntegratedDemoHandler {
    node_name: String,
    high_priority_count: AtomicUsize,
    low_priority_count: AtomicUsize,
    transaction_count: AtomicUsize,
    identity_count: AtomicUsize,
    other_count: AtomicUsize,
}

impl IntegratedDemoHandler {
    fn new(node_name: &str) -> Self {
        Self {
            node_name: node_name.to_string(),
            high_priority_count: AtomicUsize::new(0),
            low_priority_count: AtomicUsize::new(0),
            transaction_count: AtomicUsize::new(0),
            identity_count: AtomicUsize::new(0),
            other_count: AtomicUsize::new(0),
        }
    }
    
    fn get_stats(&self) -> (usize, usize, usize, usize, usize) {
        (
            self.high_priority_count.load(Ordering::Relaxed),
            self.low_priority_count.load(Ordering::Relaxed),
            self.transaction_count.load(Ordering::Relaxed),
            self.identity_count.load(Ordering::Relaxed),
            self.other_count.load(Ordering::Relaxed),
        )
    }
    
    fn log_stats(&self) {
        let (high, low, tx, id, other) = self.get_stats();
        info!(
            "[{}] Message statistics: high={}, low={}, tx={}, id={}, other={}",
            self.node_name, high, low, tx, id, other
        );
    }
}

#[async_trait]
impl MessageHandler for IntegratedDemoHandler {
    fn id(&self) -> usize {
        0
    }
    
    fn name(&self) -> &str {
        &self.node_name
    }
    
    async fn handle_message(&self, message: &NetworkMessage, peer: &PeerInfo) -> NetworkResult<()> {
        let peer_id = match libp2p::PeerId::from_bytes(&peer.id) {
            Ok(id) => id.to_string(),
            Err(_) => "unknown".to_string(),
        };
        
        // Update message type counters
        match message {
            NetworkMessage::TransactionAnnouncement(tx) => {
                self.transaction_count.fetch_add(1, Ordering::Relaxed);
                
                // Check if this is a high priority message from content
                if tx.transaction_id.contains("high_priority") {
                    info!("[{}] Received HIGH PRIORITY transaction from {}: {}", 
                          self.node_name, peer_id, tx.transaction_id);
                    self.high_priority_count.fetch_add(1, Ordering::Relaxed);
                } else {
                    debug!("[{}] Received LOW PRIORITY transaction from {}: {}", 
                          self.node_name, peer_id, tx.transaction_id);
                    self.low_priority_count.fetch_add(1, Ordering::Relaxed);
                }
                
                // Simulate processing time difference
                if tx.transaction_id.contains("low_priority") {
                    time::sleep(Duration::from_millis(30)).await;
                } else {
                    time::sleep(Duration::from_millis(5)).await;
                }
            },
            NetworkMessage::IdentityAnnouncement(_) => {
                self.identity_count.fetch_add(1, Ordering::Relaxed);
                debug!("[{}] Received identity announcement from {}", self.node_name, peer_id);
                time::sleep(Duration::from_millis(10)).await;
            },
            _ => {
                self.other_count.fetch_add(1, Ordering::Relaxed);
                debug!("[{}] Received other message from {}: {:?}", self.node_name, peer_id, message);
                time::sleep(Duration::from_millis(15)).await;
            }
        }
        
        Ok(())
    }
}

async fn run_metrics_demo(handler: Arc<IntegratedDemoHandler>, network1: Arc<P2pNetwork>, network2: Arc<P2pNetwork>) -> anyhow::Result<()> {
    info!("=== Running metrics demo ===");
    
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    
    info!("Simulating network activity for metrics collection...");
    
    // Send various types of messages
    for i in 0..20 {
        let tx = TransactionAnnouncement {
            transaction_id: format!("metrics_demo_tx_{}", i),
            transaction_type: "transfer".to_string(),
            timestamp: i as u64,
            sender: "metrics_demo".to_string(),
            data_hash: "deadbeef".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
        
        // Small delay between messages
        time::sleep(Duration::from_millis(50)).await;
    }
    
    // Wait for metrics to be collected
    time::sleep(Duration::from_secs(1)).await;
    
    // Log stats
    handler.log_stats();
    
    // Output metrics info
    info!("Metrics are available at the configured metrics address");
    info!("You can visit this URL in your browser to see the metrics in real-time");
    
    info!("=== Metrics demo completed ===");
    Ok(())
}

async fn run_reputation_demo(handler: Arc<IntegratedDemoHandler>, network1: Arc<P2pNetwork>, network2: Arc<P2pNetwork>) -> anyhow::Result<()> {
    info!("=== Running reputation demo ===");
    
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    
    // Get the reputation manager
    let reputation = network1.reputation_manager().unwrap();
    
    // First, establish baseline reputation
    info!("Establishing baseline reputation for peer {}", peer_id2);
    for _ in 0..3 {
        reputation.record_change(&peer_id2, ReputationChange::ConnectionEstablished).await?;
    }
    
    // Check initial reputation
    if let Some(rep) = reputation.get_reputation(&peer_id2).await {
        info!("Initial reputation score for peer {}: {}", peer_id2, rep.score());
    }
    
    // Simulate good behavior
    info!("Simulating good behavior from peer {}", peer_id2);
    for _ in 0..5 {
        reputation.record_change(&peer_id2, ReputationChange::MessageSuccess).await?;
        reputation.record_change(&peer_id2, ReputationChange::FastResponse).await?;
        
        // Send some valid messages
        let tx = TransactionAnnouncement {
            transaction_id: "valid_transaction".to_string(),
            transaction_type: "transfer".to_string(),
            timestamp: 123456789,
            sender: "good_peer".to_string(),
            data_hash: "abcdef1234567890".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
        
        time::sleep(Duration::from_millis(100)).await;
    }
    
    // Check reputation after good behavior
    if let Some(rep) = reputation.get_reputation(&peer_id2).await {
        info!("Reputation after good behavior: {}", rep.score());
    }
    
    // Simulate bad behavior
    info!("Simulating some bad behavior from peer {}", peer_id2);
    for _ in 0..3 {
        reputation.record_change(&peer_id2, ReputationChange::SlowResponse).await?;
        reputation.record_change(&peer_id2, ReputationChange::InvalidMessage).await?;
        
        time::sleep(Duration::from_millis(100)).await;
    }
    
    // Check reputation after bad behavior
    if let Some(rep) = reputation.get_reputation(&peer_id2).await {
        info!("Reputation after mixed behavior: {}", rep.score());
    }
    
    // Simulate reputation decay
    info!("Simulating reputation decay over time");
    reputation.process_decay().await?;
    
    // Check final reputation
    if let Some(rep) = reputation.get_reputation(&peer_id2).await {
        info!("Final reputation score after decay: {}", rep.score());
    }
    
    // Check if the peer would be considered "good"
    let is_good = if let Some(rep) = reputation.get_reputation(&peer_id2).await {
        rep.score() > 25 // Default good_threshold
    } else {
        false
    };
    
    info!("Peer {} is considered a 'good' peer: {}", peer_id2, is_good);
    
    info!("=== Reputation demo completed ===");
    Ok(())
}

async fn run_priority_demo(handler: Arc<IntegratedDemoHandler>, network1: Arc<P2pNetwork>, network2: Arc<P2pNetwork>) -> anyhow::Result<()> {
    info!("=== Running priority messaging demo ===");
    
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    
    // Get the reputation manager
    let reputation = network1.reputation_manager().unwrap();
    
    // Build up reputation for peer2
    info!("Building up reputation for peer {}", peer_id2);
    for _ in 0..5 {
        reputation.record_change(&peer_id2, ReputationChange::MessageSuccess).await?;
    }
    
    // Check queue stats before sending messages
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Initial queue stats - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Send a mix of high and low priority messages
    info!("Sending mixed priority messages...");
    
    // Reset handler stats
    handler.high_priority_count.store(0, Ordering::Relaxed);
    handler.low_priority_count.store(0, Ordering::Relaxed);
    
    // First batch: low priority messages
    info!("Sending 10 low priority messages");
    for i in 0..10 {
        let tx = TransactionAnnouncement {
            transaction_id: format!("low_priority_tx_{}", i),
            transaction_type: "transfer".to_string(),
            timestamp: i as u64,
            sender: "priority_demo".to_string(),
            data_hash: "0xdeadbeef".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
        
        // Small delay between messages
        time::sleep(Duration::from_millis(10)).await;
    }
    
    // Quick check of queue after first batch
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Queue after low priority batch - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Second batch: burst of mixed messages (5 high, 15 low, interleaved)
    info!("Sending 20 mixed priority messages (5 high, 15 low)");
    for i in 0..20 {
        let is_high_priority = i % 4 == 0; // Every 4th message is high priority
        
        let tx = TransactionAnnouncement {
            transaction_id: if is_high_priority { 
                format!("high_priority_tx_{}", i / 4) 
            } else { 
                format!("low_priority_tx_{}", 10 + i) 
            },
            transaction_type: "transfer".to_string(),
            timestamp: (i + 10) as u64,
            sender: "priority_demo".to_string(),
            data_hash: "0xdeadbeef".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network2.send_to(&peer_id1, message).await?;
        
        // No delay to test priority under load
    }
    
    // Check queue immediately after burst
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Queue right after mixed burst - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Wait a moment for some processing
    time::sleep(Duration::from_millis(100)).await;
    
    // Check intermediate processing - high priority should be processed first
    handler.log_stats();
    
    // Wait longer for all messages to be processed
    time::sleep(Duration::from_secs(2)).await;
    
    // Final queue check
    let (size, highest, lowest) = network1.get_message_queue_stats().await?;
    info!("Final queue state - Size: {}, Highest: {:?}, Lowest: {:?}", 
          size, highest, lowest);
    
    // Final stats
    handler.log_stats();
    
    info!("=== Priority messaging demo completed ===");
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();
    
    // Initialize logging
    let level = if args.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    info!("Starting ICN Network Integrated Demo");
    info!("Metrics will be available at: {}", args.metrics_address);
    
    // Create storage instances
    let storage1 = Arc::new(MockStorage::new());
    let storage2 = Arc::new(MockStorage::new());
    
    // Configure first node (full features)
    let mut config1 = P2pConfig::default();
    config1.listen_addresses = vec!["/ip4/127.0.0.1/tcp/10201".parse()?];
    config1.enable_metrics = true;
    config1.metrics_address = Some(args.metrics_address.clone());
    config1.enable_reputation = true;
    config1.enable_message_prioritization = true;
    
    // Custom reputation config
    let reputation_config = ReputationConfig {
        ban_threshold: -50,
        good_threshold: 25,
        ..ReputationConfig::default()
    };
    config1.reputation_config = Some(reputation_config);
    
    // Custom priority config
    let mut priority_config = messaging::PriorityConfig::default();
    priority_config.mode = messaging::PriorityMode::TypeAndReputation;
    priority_config.high_priority_reputation = 10; // Lower threshold for demo
    config1.priority_config = Some(priority_config);
    
    // Configure second node (simpler)
    let mut config2 = P2pConfig::default();
    config2.listen_addresses = vec!["/ip4/127.0.0.1/tcp/10202".parse()?];
    
    // Create networks
    let network1 = Arc::new(P2pNetwork::new(storage1, config1).await?);
    let network2 = Arc::new(P2pNetwork::new(storage2, config2).await?);
    
    // Create and register handlers
    let handler1 = Arc::new(IntegratedDemoHandler::new("Node1"));
    
    network1.register_message_handler("ledger.transaction", handler1.clone()).await?;
    
    // Start networks
    info!("Starting network nodes...");
    network1.start().await?;
    network2.start().await?;
    
    // Get peer IDs
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    
    info!("Node 1 (feature-rich) peer ID: {}", peer_id1);
    info!("Node 2 (basic) peer ID: {}", peer_id2);
    
    // Connect nodes
    info!("Connecting Node 2 to Node 1...");
    network2.connect(&format!("/ip4/127.0.0.1/tcp/10201/p2p/{}", peer_id1)).await?;
    
    // Wait for connection to establish
    time::sleep(Duration::from_secs(1)).await;
    
    // Run demos based on command
    match args.command {
        Command::All => {
            run_metrics_demo(handler1.clone(), network1.clone(), network2.clone()).await?;
            time::sleep(Duration::from_secs(1)).await;
            
            run_reputation_demo(handler1.clone(), network1.clone(), network2.clone()).await?;
            time::sleep(Duration::from_secs(1)).await;
            
            run_priority_demo(handler1.clone(), network1.clone(), network2.clone()).await?;
        },
        Command::Metrics => {
            run_metrics_demo(handler1.clone(), network1.clone(), network2.clone()).await?;
        },
        Command::Reputation => {
            run_reputation_demo(handler1.clone(), network1.clone(), network2.clone()).await?;
        },
        Command::Priority => {
            run_priority_demo(handler1.clone(), network1.clone(), network2.clone()).await?;
        },
    }
    
    info!("Demo completed. Stopping networks...");
    
    // Clean up
    network1.stop().await?;
    network2.stop().await?;
    
    info!("All done!");
    Ok(())
} 