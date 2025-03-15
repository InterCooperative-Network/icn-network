use std::sync::Arc;
use std::time::Duration;
use async_trait::async_trait;
use futures::StreamExt;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, MessageHandler, 
    PeerInfo, NetworkResult, NetworkMessage, CircuitRelayConfig,
    TransactionAnnouncement
};
use icn_core::storage::MockStorage;
use libp2p::Multiaddr;
use tracing_subscriber::FmtSubscriber;
use tokio::time;
use tokio::sync::oneshot;
use tracing::{info, warn, error, debug};
use clap::{Parser, Subcommand};

/// Circuit relay demo for ICN Network
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
    /// Run the circuit relay server node
    RelayServer {
        /// Port to listen on
        #[clap(short, long, default_value = "9000")]
        port: u16,
    },
    
    /// Run a node directly accessible from the internet
    PublicNode {
        /// Port to listen on
        #[clap(short, long, default_value = "9001")]
        port: u16,
        
        /// Relay server address to connect to
        #[clap(short, long)]
        relay: String,
    },
    
    /// Run a private node (behind NAT)
    PrivateNode {
        /// Relay server address to connect to
        #[clap(short, long)]
        relay: String,
        
        /// Public node peer ID to connect to
        #[clap(short, long)]
        target: String,
    },
}

/// Simple message handler for the demo
struct RelayDemoHandler {
    node_type: String,
}

impl RelayDemoHandler {
    fn new(node_type: &str) -> Self {
        Self {
            node_type: node_type.to_string(),
        }
    }
}

#[async_trait]
impl MessageHandler for RelayDemoHandler {
    fn id(&self) -> usize {
        0
    }
    
    fn name(&self) -> &str {
        "relay_demo_handler"
    }
    
    async fn handle_message(&self, message: &NetworkMessage, peer: &PeerInfo) -> NetworkResult<()> {
        if let Ok(peer_id) = libp2p::PeerId::from_bytes(&peer.id) {
            info!("[{}] Received message from {}: {:?}", 
                  self.node_type, peer_id, message);
        }
        
        Ok(())
    }
}

/// Run a relay server node
async fn run_relay_server(port: u16, metrics_address: String) -> anyhow::Result<()> {
    info!("Starting relay server node on port {}", port);
    
    // Create storage
    let storage = Arc::new(MockStorage::new());
    
    // Configure the relay server
    let mut config = P2pConfig::default();
    config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", port).parse()?];
    config.enable_metrics = true;
    config.metrics_address = Some(metrics_address);
    config.enable_circuit_relay = true;
    
    // Create relay server configuration
    let mut relay_config = CircuitRelayConfig::default();
    relay_config.enable_relay_server = true;
    relay_config.enable_relay_client = true;
    config.circuit_relay_config = Some(relay_config);
    
    // Create and start the network
    let network = Arc::new(P2pNetwork::new(storage, config).await?);
    network.start().await?;
    
    // Get the server's peer ID and addresses
    let peer_id = network.local_peer_id()?;
    let listen_addrs = network.listen_addresses().await?;
    
    info!("Relay server started");
    info!("Relay server peer ID: {}", peer_id);
    info!("Relay server addresses:");
    
    for addr in listen_addrs {
        info!("  {}/p2p/{}", addr, peer_id);
    }
    
    // Keep the server running until Ctrl+C
    let (tx, rx) = oneshot::channel();
    
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down relay server...");
        }
        _ = rx => {
            info!("Received shutdown signal, stopping relay server...");
        }
    }
    
    // Stop the network and exit
    network.stop().await?;
    info!("Relay server stopped");
    
    Ok(())
}

/// Run a public node (directly accessible)
async fn run_public_node(port: u16, relay_addr: &str, metrics_address: String) -> anyhow::Result<()> {
    info!("Starting public node on port {} with relay {}", port, relay_addr);
    
    // Create storage
    let storage = Arc::new(MockStorage::new());
    
    // Configure the public node
    let mut config = P2pConfig::default();
    config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", port).parse()?];
    config.enable_metrics = true;
    config.metrics_address = Some(metrics_address);
    config.enable_circuit_relay = true;
    
    // Create relay client configuration
    let mut relay_config = CircuitRelayConfig::default();
    relay_config.enable_relay_server = false; // Not a relay, just a client
    relay_config.enable_relay_client = true;
    relay_config.known_relay_servers = vec![relay_addr.parse()?];
    config.circuit_relay_config = Some(relay_config);
    
    // Create and start the network
    let network = Arc::new(P2pNetwork::new(storage, config).await?);
    
    // Register a message handler
    let handler = Arc::new(RelayDemoHandler::new("PublicNode"));
    network.register_message_handler("demo.message", handler).await?;
    
    network.start().await?;
    
    // Get the node's peer ID and addresses
    let peer_id = network.local_peer_id()?;
    let listen_addrs = network.listen_addresses().await?;
    
    info!("Public node started");
    info!("Public node peer ID: {}", peer_id);
    info!("Public node addresses:");
    
    for addr in listen_addrs {
        info!("  {}/p2p/{}", addr, peer_id);
    }
    
    // Connect to the relay server
    info!("Connecting to relay server at {}", relay_addr);
    let relay_addr: Multiaddr = relay_addr.parse()?;
    network.connect(&relay_addr).await?;
    
    info!("Connected to relay server");
    
    // Keep the node running until Ctrl+C
    let (tx, rx) = oneshot::channel();
    
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Received Ctrl+C, shutting down public node...");
        }
        _ = rx => {
            info!("Received shutdown signal, stopping public node...");
        }
    }
    
    // Stop the network and exit
    network.stop().await?;
    info!("Public node stopped");
    
    Ok(())
}

/// Run a private node (behind NAT)
async fn run_private_node(relay_addr: &str, target_peer: &str, metrics_address: String) -> anyhow::Result<()> {
    info!("Starting private node using relay {} to connect to {}", relay_addr, target_peer);
    
    // Parse the target peer ID
    let target_peer_id = target_peer.parse()?;
    
    // Create storage
    let storage = Arc::new(MockStorage::new());
    
    // Configure the private node
    let mut config = P2pConfig::default();
    config.listen_addresses = vec!["/ip4/127.0.0.1/tcp/0".parse()?]; // Ephemeral port, only local
    config.enable_metrics = true;
    config.metrics_address = Some(metrics_address);
    config.enable_circuit_relay = true;
    
    // Create relay client configuration
    let mut relay_config = CircuitRelayConfig::default();
    relay_config.enable_relay_server = false; // Not a relay, just a client
    relay_config.enable_relay_client = true;
    relay_config.known_relay_servers = vec![relay_addr.parse()?];
    config.circuit_relay_config = Some(relay_config);
    
    // Create and start the network
    let network = Arc::new(P2pNetwork::new(storage, config).await?);
    
    // Register a message handler
    let handler = Arc::new(RelayDemoHandler::new("PrivateNode"));
    network.register_message_handler("demo.message", handler).await?;
    
    network.start().await?;
    
    // Get the node's peer ID
    let peer_id = network.local_peer_id()?;
    
    info!("Private node started");
    info!("Private node peer ID: {}", peer_id);
    
    // Connect to the relay server first
    info!("Connecting to relay server at {}", relay_addr);
    let relay_addr: Multiaddr = relay_addr.parse()?;
    network.connect(&relay_addr).await?;
    
    info!("Connected to relay server");
    
    // Wait a bit before connecting to the target
    time::sleep(Duration::from_secs(2)).await;
    
    // Connect to the target peer through the relay
    info!("Connecting to target peer {} via relay", target_peer);
    network.smart_connect(&target_peer_id).await?;
    
    info!("Connected to target peer via relay");
    
    // Send a message to the target peer
    info!("Sending messages to target peer");
    
    let mut counter = 0;
    loop {
        let tx = TransactionAnnouncement {
            transaction_id: format!("relay_test_tx_{}", counter),
            transaction_type: "transfer".to_string(),
            timestamp: counter as u64,
            sender: "private_node".to_string(),
            data_hash: "relayed_message".to_string(),
        };
        
        let message = NetworkMessage::TransactionAnnouncement(tx);
        network.send_to(&target_peer_id, message).await?;
        
        info!("Sent message {} via relay", counter);
        counter += 1;
        
        // Check if we should exit
        if tokio::signal::ctrl_c().now_or_never().is_some() {
            break;
        }
        
        // Wait before sending the next message
        time::sleep(Duration::from_secs(5)).await;
    }
    
    info!("Received Ctrl+C, shutting down private node...");
    
    // Stop the network and exit
    network.stop().await?;
    info!("Private node stopped");
    
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
    
    info!("Starting ICN Network Circuit Relay Demo");
    
    // Run the appropriate node type
    match args.command {
        Command::RelayServer { port } => {
            run_relay_server(port, args.metrics_address).await?;
        },
        Command::PublicNode { port, relay } => {
            run_public_node(port, &relay, args.metrics_address).await?;
        },
        Command::PrivateNode { relay, target } => {
            run_private_node(&relay, &target, args.metrics_address).await?;
        },
    }
    
    info!("Demo completed!");
    Ok(())
} 