// ICN Node entry point

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use clap::Parser;
use config::{Config, File};
use icn_network::{P2pConfig, P2pNetwork, NetworkMessage, NetworkService};
use libp2p::Multiaddr;
use serde::Deserialize;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// ICN Node CLI arguments
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to node configuration file
    #[clap(short, long, value_parser)]
    config: Option<PathBuf>,

    /// Path to network configuration file
    #[clap(short, long, value_parser)]
    network_config: Option<PathBuf>,

    /// Node ID
    #[clap(short, long)]
    node_id: Option<String>,

    /// Listen address
    #[clap(short, long)]
    listen_addr: Option<String>,

    /// Log level
    #[clap(long, default_value = "info")]
    log_level: String,

    /// Path to data directory
    #[clap(long)]
    data_dir: Option<PathBuf>,
}

/// Node configuration
#[derive(Debug, Deserialize)]
struct NodeConfig {
    /// Node identifier
    node_id: String,
    /// Node type
    node_type: String,
    /// Listen address
    listen_addr: String,
    /// Optional peer ID
    peer_id: Option<String>,
    /// Path to data directory
    data_dir: String,
    /// Path to log directory
    log_dir: String,
    /// Path to network configuration
    network_config_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Set up logging
    let log_level = match args.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting ICN Node...");
    
    // Load configuration
    let config_path = args.config.unwrap_or_else(|| PathBuf::from("config/node.yaml"));
    info!("Loading configuration from {}", config_path.display());
    
    let config = Config::builder()
        .add_source(File::from(config_path).required(true))
        .build()
        .context("Failed to load configuration")?;
    
    let node_config: NodeConfig = config.try_deserialize()
        .context("Failed to parse node configuration")?;
    
    // Load network configuration
    let network_config_path = args.network_config
        .unwrap_or_else(|| PathBuf::from(&node_config.network_config_path));
    info!("Loading network configuration from {}", network_config_path.display());
    
    let network_config = Config::builder()
        .add_source(File::from(network_config_path).required(true))
        .build()
        .context("Failed to load network configuration")?;
    
    // Initialize P2P network
    info!("Initializing P2P network...");
    
    // Extract listen address from config or command line
    let listen_addr = args.listen_addr
        .unwrap_or_else(|| node_config.listen_addr.clone());
    
    // Convert to multiaddr
    let listen_multiaddr: Multiaddr = listen_addr.parse()
        .context("Failed to parse listen address")?;
    
    // Ensure primary listen address is set correctly
    let mut p2p_config = P2pConfig::default();
    p2p_config.listen_addresses = vec![
        listen_multiaddr,
        // Also listen on IPv6 if the address is IPv4 (and vice versa)
        if listen_addr.contains("/ip4/") {
            "/ip6/::/tcp/9000".parse().unwrap()
        } else {
            "/ip4/0.0.0.0/tcp/9000".parse().unwrap()
        }
    ];
    
    // Extract other p2p config from network configuration
    if let Ok(p2p) = network_config.get_table("p2p") {
        if let Some(bs_peers) = p2p.get("bootstrap_peers") {
            if let Ok(peers) = bs_peers.clone().into_array() {
                let bootstrap_peers = peers.into_iter()
                    .filter_map(|p| p.into_string().ok())
                    .collect::<Vec<_>>();
                p2p_config.bootstrap_peers = bootstrap_peers;
            }
        }
        
        if let Some(enable_mdns) = p2p.get("enable_mdns") {
            if let Ok(val) = enable_mdns.clone().into_bool() {
                p2p_config.enable_mdns = val;
            }
        }
        
        if let Some(enable_kademlia) = p2p.get("enable_kademlia") {
            if let Ok(val) = enable_kademlia.clone().into_bool() {
                p2p_config.enable_kademlia = val;
            }
        }
        
        if let Some(enable_metrics) = p2p.get("enable_metrics") {
            if let Ok(val) = enable_metrics.clone().into_bool() {
                p2p_config.enable_metrics = val;
            }
        }
        
        if let Some(metrics_addr) = p2p.get("metrics_address") {
            if let Ok(val) = metrics_addr.clone().into_string() {
                p2p_config.metrics_address = Some(val);
            }
        }
        
        if let Some(enable_rep) = p2p.get("enable_reputation") {
            if let Ok(val) = enable_rep.clone().into_bool() {
                p2p_config.enable_reputation = val;
            }
        }
        
        if let Some(enable_relay) = p2p.get("enable_circuit_relay") {
            if let Ok(val) = enable_relay.clone().into_bool() {
                p2p_config.enable_circuit_relay = val;
            }
        }
    }
    
    // Initialize the storage system
    let data_dir = args.data_dir.map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| node_config.data_dir.clone());
    
    // Set peer store path
    p2p_config.peer_store_path = Some(format!("{}/peers", data_dir));
    
    // Create storage
    let storage = Arc::new(
        icn_core::storage::FileStorage::new(&data_dir)
            .await
            .context("Failed to initialize storage")?
    );
    
    // Create and start the network
    let p2p_network = P2pNetwork::new(
        storage.clone(),
        p2p_config,
    )
    .await
    .context("Failed to create P2P network")?;
    
    let message_receiver = p2p_network.subscribe_messages().await
        .context("Failed to subscribe to network messages")?;
    
    let network_handle = p2p_network.start().await
        .context("Failed to start P2P network")?;
    
    // Create a message handling task
    let message_handler = tokio::spawn(async move {
        handle_messages(message_receiver).await;
    });
    
    info!("Node initialized, press Ctrl+C to exit");
    
    // Wait for shutdown signal
    signal::ctrl_c().await?;
    info!("Shutting down ICN Node...");
    
    // Clean shutdown
    p2p_network.stop().await?;
    
    // Wait for message handler to complete
    if let Err(e) = message_handler.await {
        error!("Message handler task failed: {}", e);
    }
    
    info!("Node shutdown complete");
    Ok(())
}

// Handle incoming network messages
async fn handle_messages(mut receiver: mpsc::Receiver<(String, NetworkMessage)>) {
    while let Some((peer_id, message)) = receiver.recv().await {
        match message {
            NetworkMessage::LedgerStateUpdate(update) => {
                debug!("Received ledger state update from {}: hash={}, tx_count={}", 
                       peer_id, update.ledger_hash, update.transaction_count);
            },
            NetworkMessage::TransactionAnnouncement(tx) => {
                debug!("Received transaction announcement from {}: id={}, type={}", 
                       peer_id, tx.transaction_id, tx.transaction_type);
            },
            NetworkMessage::IdentityAnnouncement(id) => {
                debug!("Received identity announcement from {}: id={}", 
                       peer_id, id.identity_id);
            },
            NetworkMessage::ProposalAnnouncement(prop) => {
                debug!("Received proposal announcement from {}: id={}, title='{}'", 
                       peer_id, prop.proposal_id, prop.title);
            },
            NetworkMessage::VoteAnnouncement(vote) => {
                debug!("Received vote announcement from {}: proposal={}, voter={}, decision={}", 
                       peer_id, vote.proposal_id, vote.voter_id, vote.decision);
            },
            NetworkMessage::Custom(custom) => {
                debug!("Received custom message from {}: type={}", 
                       peer_id, custom.message_type);
            },
        }
    }
    info!("Message handler shutting down");
} 