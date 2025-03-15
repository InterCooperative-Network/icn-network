use std::sync::Arc;
use std::time::Duration;

use clap::{Parser, Subcommand};
use icn_core::storage::mock_storage::MockStorage;
use icn_network::{
    MessageProcessor, NetworkMessage, NetworkService, P2pConfig, P2pNetwork,
    DefaultMessageHandler, TransactionAnnouncement, PeerInfo,
};
use libp2p::Multiaddr;
use tokio::sync::mpsc;
use tokio::time::sleep;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// A simple command-line interface for testing the ICN network layer
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port to listen on for P2P connections
    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    /// Enable mDNS discovery
    #[arg(short, long, default_value_t = true)]
    mdns: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Start a node and wait for connections
    Listen {
        /// Node name for logging
        #[arg(short, long, default_value = "node")]
        name: String,
    },
    
    /// Start a node and connect to a peer
    Connect {
        /// Node name for logging
        #[arg(short, long, default_value = "node")]
        name: String,
        
        /// The multiaddress of the peer to connect to
        #[arg(short, long)]
        peer: String,
    },
    
    /// Start a node and broadcast a message periodically
    Broadcast {
        /// Node name for logging
        #[arg(short, long, default_value = "node")]
        name: String,
        
        /// The multiaddress of the peer to connect to
        #[arg(short, long)]
        peer: Option<String>,
        
        /// The interval in seconds between broadcasts
        #[arg(short, long, default_value_t = 5)]
        interval: u64,
        
        /// The number of messages to send (0 = infinite)
        #[arg(short, long, default_value_t = 0)]
        count: u32,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,icn_network=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Create a mock storage for this example
    let storage = Arc::new(MockStorage::new());
    
    // Configure the network
    let mut config = P2pConfig::default();
    config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", args.port).parse()?];
    config.enable_mdns = args.mdns;
    
    // Create the network
    let network = Arc::new(P2pNetwork::new(storage, config).await?);
    
    // Create a channel for user input
    let (tx, mut rx) = mpsc::channel(1);
    let tx_clone = tx.clone();
    
    // Handle Ctrl+C
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        let _ = tx_clone.send("quit".to_string()).await;
    });
    
    // Register message handler
    let handler = Arc::new(DefaultMessageHandler::new(
        1,
        "CLI Handler".to_string(),
        move |message, peer_id| {
            match message {
                NetworkMessage::TransactionAnnouncement(tx) => {
                    tracing::info!(
                        "Received TX announcement from {}: {} ({})",
                        peer_id, tx.transaction_id, tx.transaction_type
                    );
                }
                NetworkMessage::IdentityAnnouncement(id) => {
                    tracing::info!(
                        "Received identity announcement from {}: {}",
                        peer_id, id.identity_id
                    );
                }
                _ => {
                    tracing::info!("Received message from {}: {:?}", peer_id, message);
                }
            }
            
            Ok(())
        }
    ));
    
    network.register_message_handler("cli.all", handler).await?;
    
    // Start the network
    network.start().await?;
    
    // Log information about the node
    let peer_id = network.local_peer_id();
    let addresses = network.listen_addresses().await?;
    
    tracing::info!("Node started with PeerID: {}", peer_id);
    for addr in addresses {
        tracing::info!("Listening on: {}/p2p/{}", addr, peer_id);
    }
    
    // Execute the specific command
    match args.command {
        Command::Listen { name } => {
            tracing::info!("Node '{}' is listening for connections", name);
            
            // Just wait for connections
            while let Some(cmd) = rx.recv().await {
                if cmd == "quit" {
                    break;
                }
            }
        }
        
        Command::Connect { name, peer } => {
            // Parse the multiaddress
            let addr: Multiaddr = peer.parse()?;
            
            tracing::info!("Node '{}' connecting to {}", name, addr);
            
            // Connect to the peer
            network.connect(&addr).await?;
            tracing::info!("Connected to {}", addr);
            
            // Wait for user to quit
            while let Some(cmd) = rx.recv().await {
                if cmd == "quit" {
                    break;
                }
            }
        }
        
        Command::Broadcast { name, peer, interval, count } => {
            // Connect to peer if provided
            if let Some(peer_addr) = peer {
                let addr: Multiaddr = peer_addr.parse()?;
                tracing::info!("Node '{}' connecting to {}", name, addr);
                network.connect(&addr).await?;
                tracing::info!("Connected to {}", addr);
            }
            
            // Broadcast messages
            let mut counter = 0;
            let max_count = if count == 0 { u32::MAX } else { count };
            
            loop {
                // Check for quit command
                if rx.try_recv().map(|cmd| cmd == "quit").unwrap_or(false) {
                    break;
                }
                
                // Create a message
                let tx_announce = TransactionAnnouncement {
                    transaction_id: format!("tx-{}-{}", name, counter),
                    transaction_type: "example".to_string(),
                    timestamp: 12345,
                    sender: name.clone(),
                    data_hash: format!("hash-{}", counter),
                };
                
                let message = NetworkMessage::TransactionAnnouncement(tx_announce);
                
                // Broadcast the message
                match network.broadcast(message).await {
                    Ok(_) => {
                        tracing::info!("Broadcast message #{}", counter);
                    }
                    Err(e) => {
                        tracing::error!("Failed to broadcast message: {}", e);
                    }
                }
                
                // Increment counter and check if we're done
                counter += 1;
                if counter >= max_count {
                    break;
                }
                
                // Wait for the next interval
                sleep(Duration::from_secs(interval)).await;
            }
        }
    }
    
    // Stop the network
    network.stop().await?;
    tracing::info!("Network stopped");
    
    Ok(())
} 