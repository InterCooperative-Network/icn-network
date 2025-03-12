use clap::Parser;
use icn_common::Result;
use icn_node_core::{Node, IcnNode, NodeConfig, NetworkMode};
use std::path::PathBuf;
use tracing::{info, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(author, version, about = "ICN Node")]
struct Args {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
    
    /// Set the network mode (standalone, local, mesh, full, relay)
    #[arg(short, long, default_value = "local")]
    network: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command-line arguments
    let args = Args::parse();
    
    // Initialize logging with dynamic filter based on verbosity
    let filter = if args.verbose {
        EnvFilter::from_default_env().add_directive(LevelFilter::DEBUG.into())
    } else {
        EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into())
    };
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .init();
    
    // Log start message
    info!("Starting ICN node");
    
    let node = if let Some(config_path) = args.config {
        // Load configuration from file
        info!("Loading configuration from {:?}", config_path);
        IcnNode::from_config_file(config_path).await?
    } else {
        // Create default configuration
        let network_mode = match args.network.to_lowercase().as_str() {
            "standalone" => NetworkMode::Standalone,
            "mesh" => NetworkMode::Mesh,
            "full" => NetworkMode::Full,
            "relay" => NetworkMode::Relay,
            _ => NetworkMode::Local,
        };
        
        info!("Using {} network mode", args.network);
        
        let config = NodeConfig::default();
        
        IcnNode::initialize(config).await?
    };
    
    // Start the node
    let mut node = node;
    node.start().await?;
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    
    // Stop the node
    info!("Stopping ICN node");
    node.stop().await?;
    
    info!("ICN node stopped");
    
    Ok(())
} 