use std::sync::Arc;
use std::time::Duration;
use clap::{Parser, Subcommand};
use tokio::signal;
use tokio::time;
use tracing::{info, warn, debug, error};
use tracing_subscriber::FmtSubscriber;
use icn_network::{
    P2pNetwork, P2pConfig, NetworkService, 
    NetworkMessage, ReputationChange
};
use icn_core::storage::MockStorage;
use libp2p::Multiaddr;

#[derive(Parser)]
#[clap(name = "icn-net", about = "ICN Network CLI tool")]
struct Cli {
    #[clap(short, long, help = "Verbose output")]
    verbose: bool,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(about = "Start a listening node")]
    Listen {
        #[clap(short, long, help = "Port to listen on", default_value = "10000")]
        port: u16,
        
        #[clap(short, long, help = "Enable metrics", default_value = "false")]
        metrics: bool,
        
        #[clap(short, long, help = "Metrics port", default_value = "9091")]
        metrics_port: u16,
    },
    
    #[clap(about = "Connect to another node")]
    Connect {
        #[clap(short, long, help = "Target peer address")]
        target: String,
        
        #[clap(short, long, help = "Local port", default_value = "0")]
        port: u16,
    },
    
    #[clap(about = "Broadcast a message")]
    Broadcast {
        #[clap(short, long, help = "Message type", default_value = "ledger.transaction")]
        r#type: String,
        
        #[clap(short, long, help = "Message content")]
        content: String,
        
        #[clap(short, long, help = "Local port", default_value = "0")]
        port: u16,
    },
    
    #[clap(about = "Start a node with metrics enabled")]
    Metrics {
        #[clap(short, long, help = "Network port", default_value = "10000")]
        port: u16,
        
        #[clap(short, long, help = "Metrics port", default_value = "9091")]
        metrics_port: u16,
    },
    
    #[clap(about = "Run the reputation system demo")]
    Reputation {
        #[clap(short, long, help = "Base port for the demo", default_value = "10000")]
        base_port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    // Initialize tracing based on verbosity
    let level = if cli.verbose {
        tracing::Level::DEBUG
    } else {
        tracing::Level::INFO
    };
    
    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    
    match cli.command {
        Commands::Listen { port, metrics, metrics_port } => {
            let storage = Arc::new(MockStorage::new());
            
            // Create network configuration
            let mut config = P2pConfig::default();
            config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", port).parse()?];
            
            // Enable metrics if requested
            if metrics {
                config.enable_metrics = true;
                config.metrics_address = Some(format!("127.0.0.1:{}", metrics_port));
                info!("Metrics enabled on port {}", metrics_port);
            }
            
            // Create and start the network
            let network = P2pNetwork::new(storage, config).await?;
            network.start().await?;
            
            info!("Node started with peer ID: {}", network.local_peer_id()?);
            info!("Listening on port {}", port);
            
            // Wait for Ctrl+C
            wait_for_shutdown().await;
            
            info!("Shutting down...");
            network.stop().await?;
        },
        
        Commands::Connect { target, port } => {
            let storage = Arc::new(MockStorage::new());
            
            // Create network configuration
            let mut config = P2pConfig::default();
            config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", port).parse()?];
            
            // Create and start the network
            let network = P2pNetwork::new(storage, config).await?;
            network.start().await?;
            
            info!("Node started with peer ID: {}", network.local_peer_id()?);
            
            // Connect to the target
            info!("Connecting to {}", target);
            network.connect(&target).await?;
            info!("Connected to {}", target);
            
            // Wait for Ctrl+C
            wait_for_shutdown().await;
            
            info!("Shutting down...");
            network.stop().await?;
        },
        
        Commands::Broadcast { r#type, content, port } => {
            let storage = Arc::new(MockStorage::new());
            
            // Create network configuration
            let mut config = P2pConfig::default();
            config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", port).parse()?];
            
            // Create and start the network
            let network = P2pNetwork::new(storage, config).await?;
            network.start().await?;
            
            info!("Node started with peer ID: {}", network.local_peer_id()?);
            
            // Wait a moment for potential connections
            time::sleep(Duration::from_secs(2)).await;
            
            // Create and broadcast the message
            let message = NetworkMessage {
                message_type: r#type.clone(),
                content: content.into_bytes(),
            };
            
            info!("Broadcasting message of type: {}", r#type);
            network.broadcast_message(message).await?;
            info!("Message broadcast complete");
            
            // Wait a moment for message delivery
            time::sleep(Duration::from_secs(2)).await;
            
            info!("Shutting down...");
            network.stop().await?;
        },
        
        Commands::Metrics { port, metrics_port } => {
            let storage = Arc::new(MockStorage::new());
            
            // Create network configuration with metrics
            let mut config = P2pConfig::default();
            config.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", port).parse()?];
            config.enable_metrics = true;
            config.metrics_address = Some(format!("127.0.0.1:{}", metrics_port));
            
            // Create and start the network
            let network = P2pNetwork::new(storage, config).await?;
            network.start().await?;
            
            info!("Node started with peer ID: {}", network.local_peer_id()?);
            info!("Metrics available at http://127.0.0.1:{}/metrics", metrics_port);
            
            // Wait for Ctrl+C
            wait_for_shutdown().await;
            
            info!("Shutting down...");
            network.stop().await?;
        },
        
        Commands::Reputation { base_port } => {
            run_reputation_demo(base_port).await?;
        },
    }
    
    Ok(())
}

/// Run the reputation system demo
async fn run_reputation_demo(base_port: u16) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting reputation system demo...");
    
    // Create storage instances
    let storage1 = Arc::new(MockStorage::new());
    let storage2 = Arc::new(MockStorage::new());
    
    // Create network configurations
    let mut config1 = P2pConfig::default();
    config1.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", base_port).parse()?];
    config1.enable_reputation = true;
    
    let mut config2 = P2pConfig::default();
    config2.listen_addresses = vec![format!("/ip4/0.0.0.0/tcp/{}", base_port + 1).parse()?];
    config2.enable_reputation = true;
    
    // Create and start the networks
    info!("Starting nodes...");
    let network1 = Arc::new(P2pNetwork::new(storage1, config1).await?);
    let network2 = Arc::new(P2pNetwork::new(storage2, config2).await?);
    
    network1.start().await?;
    network2.start().await?;
    
    let peer_id1 = network1.local_peer_id()?;
    let peer_id2 = network2.local_peer_id()?;
    
    info!("Node 1 peer ID: {}", peer_id1);
    info!("Node 2 peer ID: {}", peer_id2);
    
    // Connect the nodes
    let addr2 = format!("/ip4/127.0.0.1/tcp/{}/p2p/{}", base_port + 1, peer_id2);
    info!("Connecting node 1 to node 2 at {}", addr2);
    network1.connect(&addr2).await?;
    
    // Wait for connection to establish
    time::sleep(Duration::from_secs(1)).await;
    
    // Demo 1: Record some positive reputation changes
    info!("=== Recording positive reputation changes ===");
    let reputation1 = network1.reputation_manager().unwrap();
    
    for i in 0..3 {
        reputation1.record_change(&peer_id2, ReputationChange::MessageSuccess).await?;
        info!("Recorded positive change {}/3", i+1);
        time::sleep(Duration::from_millis(500)).await;
    }
    
    // Check the reputation
    let rep = reputation1.get_reputation(&peer_id2).await;
    if let Some(rep) = rep {
        info!("Node 2 reputation after positive changes: {}", rep.score());
    }
    
    // Demo 2: Record some negative reputation changes
    info!("=== Recording negative reputation changes ===");
    
    for i in 0..2 {
        reputation1.record_change(&peer_id2, ReputationChange::MessageFailure).await?;
        info!("Recorded negative change {}/2", i+1);
        time::sleep(Duration::from_millis(500)).await;
    }
    
    // Check the reputation again
    let rep = reputation1.get_reputation(&peer_id2).await;
    if let Some(rep) = rep {
        info!("Node 2 reputation after negative changes: {}", rep.score());
    }
    
    // Demo 3: Ban and unban
    info!("=== Ban and unban demonstration ===");
    
    info!("Banning node 2...");
    network1.ban_peer(&peer_id2).await?;
    
    // Check ban status
    let is_banned = reputation1.is_banned(&peer_id2).await;
    info!("Node 2 banned status: {}", is_banned);
    
    // Try to reconnect (should fail or be ignored due to ban)
    let result = network1.connect(&addr2).await;
    match result {
        Ok(_) => info!("Connect succeeded but peer is still banned"),
        Err(e) => info!("Connect failed as expected: {}", e),
    }
    
    // Unban
    info!("Unbanning node 2...");
    network1.unban_peer(&peer_id2).await?;
    
    // Check ban status again
    let is_banned = reputation1.is_banned(&peer_id2).await;
    info!("Node 2 banned status after unban: {}", is_banned);
    
    // Reconnect after unban
    info!("Reconnecting to node 2...");
    network1.connect(&addr2).await?;
    
    // Final reputation check
    let rep = reputation1.get_reputation(&peer_id2).await;
    if let Some(rep) = rep {
        info!("Node 2 final reputation: {}", rep.score());
    }
    
    info!("Reputation demo complete!");
    info!("Press Ctrl+C to exit...");
    
    // Wait for Ctrl+C
    wait_for_shutdown().await;
    
    // Clean shutdown
    info!("Shutting down...");
    network1.stop().await?;
    network2.stop().await?;
    
    Ok(())
}

async fn wait_for_shutdown() {
    match signal::ctrl_c().await {
        Ok(()) => info!("Received shutdown signal"),
        Err(err) => error!("Unable to listen for shutdown signal: {}", err),
    }
} 