use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use futures::StreamExt;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, MessageHandler, 
    PeerInfo, NetworkResult, NetworkMessage, CircuitRelayConfig,
    TransactionAnnouncement, ReputationConfig, NetworkMetrics, Timer,
    messaging::PriorityConfig, messaging::PriorityMode,
};
use libp2p::Multiaddr;
use tracing_subscriber::FmtSubscriber;
use tokio::time;
use tokio::sync::oneshot;
use tracing::{info, warn, error, debug};
use clap::{Parser, Subcommand};

/// Integrated demo for ICN Network
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Enable verbose logging
    #[clap(short, long)]
    verbose: bool,

    /// Metrics server address
    #[clap(short, long, default_value = "[::1]:9090")]
    metrics_address: String,

    /// Demo mode to run
    #[clap(default_value = "all")]
    mode: String,
}

/// Simple message handler for the demo
struct IntegratedDemoHandler {
    name: String,
}

impl IntegratedDemoHandler {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
        }
    }
}

#[async_trait]
impl MessageHandler for IntegratedDemoHandler {
    fn id(&self) -> usize {
        0
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    async fn handle_message(&self, message: &NetworkMessage, peer: &PeerInfo) -> NetworkResult<()> {
        info!("[{}] Received message from {}: {:?}", 
              self.name, peer.peer_id, message);
        
        Ok(())
    }
}

/// Run the integrated demo
async fn run_integrated_demo(args: Args) -> anyhow::Result<()> {
    info!("Starting integrated demo with mode: {}", args.mode);
    
    // Create storage (use MockStorage for this demo)
    let storage = Arc::new(icn_core::storage::MockStorage::new());
    
    // Configure the network
    let mut config = P2pConfig::default();
    config.listen_addresses = vec!["/ip6/::1/tcp/0".parse()?]; // Use ephemeral port
    config.enable_metrics = true;
    config.metrics_address = Some(args.metrics_address.clone());
    
    // Configure features based on mode
    match args.mode.as_str() {
        "metrics" | "all" => {
            info!("Enabling metrics...");
            config.enable_metrics = true;
            config.metrics_address = Some(args.metrics_address);
        },
        _ => {}
    }
    
    match args.mode.as_str() {
        "reputation" | "all" => {
            info!("Enabling reputation system...");
            config.enable_reputation = true;
            
            let reputation_config = ReputationConfig {
                ban_threshold: -50,
                decay_factor: 0.05,
                decay_interval: Duration::from_secs(300),
                ..Default::default()
            };
            config.reputation_config = Some(reputation_config);
        },
        _ => {}
    }
    
    match args.mode.as_str() {
        "priority" | "all" => {
            info!("Enabling priority messaging...");
            config.enable_message_prioritization = true;
            
            let priority_config = PriorityConfig {
                mode: PriorityMode::TypeAndReputation,
                high_priority_message_types: vec!["consensus.vote".to_string()],
                ..Default::default()
            };
            config.priority_config = Some(priority_config);
        },
        _ => {}
    }
    
    match args.mode.as_str() {
        "relay" | "all" => {
            info!("Enabling circuit relay...");
            config.enable_circuit_relay = true;
            
            let relay_config = CircuitRelayConfig {
                enable_relay_server: true,
                enable_relay_client: true,
                ..Default::default()
            };
            config.circuit_relay_config = Some(relay_config);
        },
        _ => {}
    }
    
    // Create and start the network
    let network = Arc::new(P2pNetwork::new(storage, config).await?);
    
    // Register message handlers
    let handler = Arc::new(IntegratedDemoHandler::new("integrated_demo"));
    network.register_message_handler("demo.message", handler.clone()).await?;
    network.register_message_handler("consensus.vote", handler.clone()).await?;
    
    // Start the network
    network.start().await?;
    
    // Get the node's peer ID and addresses
    let peer_id = network.local_peer_id()?;
    let listen_addrs = network.listen_addresses().await?;
    
    info!("Integrated demo node started");
    info!("Node peer ID: {}", peer_id);
    info!("Node addresses:");
    
    for addr in listen_addrs {
        info!("  {}/p2p/{}", addr, peer_id);
    }
    
    // Start simulation loop
    let mut interval = time::interval(Duration::from_secs(5));
    let mut counter = 0;
    
    // Run until Ctrl+C
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
                break;
            }
            _ = interval.tick() => {
                // Simulate network activity
                if counter % 2 == 0 {
                    // Broadcast a transaction announcement
                    let tx = TransactionAnnouncement {
                        transaction_id: format!("test_tx_{}", counter),
                        transaction_type: "transfer".to_string(),
                        timestamp: counter as u64,
                        sender: "integrated_demo".to_string(),
                        data_hash: "test_hash".to_string(),
                    };
                    
                    let message = NetworkMessage::TransactionAnnouncement(tx);
                    if let Err(e) = network.broadcast(message).await {
                        error!("Failed to broadcast message: {}", e);
                    } else {
                        info!("Broadcast transaction #{}", counter);
                    }
                } else {
                    // Broadcast a consensus vote (high priority)
                    let vote = icn_network::VoteAnnouncement {
                        proposal_id: format!("proposal_{}", counter / 2),
                        voter_id: peer_id.to_string(),
                        decision: "approve".to_string(),
                        timestamp: counter as u64,
                        data_hash: "vote_hash".to_string(),
                    };
                    
                    let message = NetworkMessage::VoteAnnouncement(vote);
                    if let Err(e) = network.broadcast(message).await {
                        error!("Failed to broadcast vote: {}", e);
                    } else {
                        info!("Broadcast vote #{}", counter / 2);
                    }
                }
                
                counter += 1;
                
                // Every 10 cycles, print some stats
                if counter % 10 == 0 {
                    if let Some(metrics) = network.metrics() {
                        info!("Network stats:");
                        info!("  Connected peers: {}", metrics.peers_connected());
                        
                        if let Some(rep_mgr) = network.reputation_manager() {
                            info!("Reputation stats:");
                            info!("  Total tracked peers: {}", rep_mgr.get_tracked_peers_count().await);
                            info!("  Banned peers: {}", rep_mgr.get_banned_peers_count().await);
                        }
                        
                        if let Ok(queue_stats) = network.get_message_queue_stats().await {
                            info!("Queue stats:");
                            info!("  Queue size: {}", queue_stats.0);
                            if let Some(highest) = queue_stats.1 {
                                info!("  Highest priority: {}", highest);
                            }
                            if let Some(lowest) = queue_stats.2 {
                                info!("  Lowest priority: {}", lowest);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Stop the network
    network.stop().await?;
    info!("Integrated demo stopped");
    
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
    
    // Run the demo
    run_integrated_demo(args).await?;
    
    info!("Demo completed!");
    Ok(())
} 