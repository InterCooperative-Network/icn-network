use icn_common::error::Result;
use icn_did::{DidManager, DidManagerConfig};
use icn_networking::{
    node::{Node, NodeConfig, NodeType, NetworkService},
    tls::TlsConfig,
    discovery::FederationInfo,
};
use icn_mutual_credit::{MutualCreditSystem, CreditLimit};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
    collections::HashMap,
    sync::Arc,
};
use tokio;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

const NUM_FEDERATIONS: usize = 2;
const NODES_PER_FEDERATION: usize = 4;
const BASE_PORT: u16 = 9001;

struct NetworkNode {
    node: Node,
    did_manager: Arc<DidManager>,
    credit_system: Arc<MutualCreditSystem>,
}

// Main testnet function
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting ICN Testnet...");
    
    // Create TLS config
    #[cfg(feature = "testing")]
    let tls_config = TlsConfig::new_self_signed("icn-testnet.local")
        .expect("Failed to create TLS config");
    
    #[cfg(not(feature = "testing"))]
    let tls_config = {
        // For non-testing environments, we need to provide paths to certificate files
        // These should be generated beforehand and placed in the specified locations
        let cert_path = "./certs/server.crt";
        let key_path = "./certs/server.key";
        
        // Check if certificate files exist, if not, create directory and generate them
        if !std::path::Path::new(cert_path).exists() || !std::path::Path::new(key_path).exists() {
            std::fs::create_dir_all("./certs").expect("Failed to create certs directory");
            
            // Generate certificate using openssl command
            let status = std::process::Command::new("openssl")
                .args(&[
                    "req", "-x509", "-newkey", "rsa:4096", 
                    "-keyout", key_path, 
                    "-out", cert_path,
                    "-days", "365", 
                    "-nodes", 
                    "-subj", "/CN=icn-testnet.local"
                ])
                .status()
                .expect("Failed to execute openssl command");
                
            if !status.success() {
                panic!("Failed to generate certificates with openssl");
            }
        }
        
        TlsConfig::new(cert_path, key_path, None::<&str>)
            .expect("Failed to create TLS config")
    };

    // Track nodes by federation
    let mut federation_nodes: HashMap<String, Vec<NodeConfig>> = HashMap::new();
    let mut port_counter = BASE_PORT;
    let mut all_nodes: Vec<NetworkNode> = Vec::new();

    // Create federations
    for fed_id in 0..NUM_FEDERATIONS {
        let federation_name = format!("federation-{}", fed_id);
        let mut nodes = Vec::new();

        // Create primary and secondary nodes for each federation
        for node_idx in 0..NODES_PER_FEDERATION {
            let node_type = if node_idx % 2 == 0 { NodeType::Primary } else { NodeType::Secondary };
            let coop_id = format!("coop-{}-{}", fed_id, node_idx / 2);
            let node_id = format!("node-{}-{}", fed_id, node_idx);
            let node_port = port_counter;
            port_counter += 1;

            // Collect peer addresses from other federations' primary nodes
            let mut peers = Vec::new();
            for (_, other_nodes) in &federation_nodes {
                if let Some(primary_node) = other_nodes.first() {
                    peers.push(primary_node.listen_addr);
                }
            }

            let config = NodeConfig {
                listen_addr: SocketAddr::new(
                    IpAddr::V4(Ipv4Addr::LOCALHOST),
                    node_port,
                ),
                peers,
                node_id: node_id.clone(),
                coop_id: coop_id.clone(),
                node_type: node_type.clone(),
                discovery_interval: Some(Duration::from_secs(30)),
                health_check_interval: Some(Duration::from_secs(10)),
            };

            // Create node
            let node = Node::new(
                node_id.clone(),
                config.listen_addr,
                tls_config.clone(),
                config.clone(),
            );
            
            nodes.push(config.clone());
            
            // Create DID manager for this node
            let did_config = DidManagerConfig {
                default_federation_id: federation_name.clone(),
            };
            let did_manager = DidManager::new(did_config).await?;
            
            // Create mutual credit system for this node
            let credit_system = Arc::new(MutualCreditSystem::new());
            
            // Store the node with its components
            all_nodes.push(NetworkNode {
                node,
                did_manager: Arc::new(did_manager),
                credit_system,
            });
        }

        federation_nodes.insert(federation_name.clone(), nodes.clone());
        
        // Create federation info
        let federation_info = FederationInfo {
            federation_id: federation_name.clone(),
            description: format!("Test Federation {}", fed_id),
            bootstrap_nodes: nodes.iter().map(|n| n.listen_addr).collect(),
            services: vec!["identity".to_string(), "credit".to_string()],
            last_seen: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        info!("Created federation: {}", federation_name);
    }

    // Set up mutual credit relationships between federations
    info!("Setting up economic relationships between federations...");
    
    // For each node, create accounts in their credit system
    for node in &all_nodes {
        // Create an account for each federation
        for (fed_name, _) in &federation_nodes {
            let account_id = format!("account-{}", fed_name);
            let _ = node.credit_system.create_account(
                account_id,
                format!("Federation Account for {}", fed_name),
                CreditLimit::new(1000),
            );
        }
    }

    // Start all nodes
    info!("Starting all nodes...");
    let mut handles = Vec::new();
    
    for (i, mut node) in all_nodes.into_iter().enumerate() {
        let node_id = format!("node-{}", i);
        
        // Start the node in a background task
        let handle = tokio::spawn(async move {
            info!("Starting node {}...", node_id);
            
            // Start the network node
            if let Err(e) = node.node.start().await {
                error!("Node {} failed: {}", node_id, e);
                return;
            }
        });
        
        handles.push(handle);
    }

    // Print network topology
    info!("\nNetwork Topology:");
    info!("================");
    for (federation_id, nodes) in &federation_nodes {
        info!("\nFederation: {}", federation_id);
        for node in nodes {
            info!("  {} ({}): {}", node.node_id, node.node_type, node.listen_addr);
            if !node.peers.is_empty() {
                info!("    Peers: {:?}", node.peers);
            }
        }
    }

    info!("\nTestnet is running. Press Ctrl+C to stop.");

    // Keep the main thread running
    tokio::signal::ctrl_c().await.expect("Failed to listen for ctrl-c");
    info!("Shutting down testnet...");
    
    Ok(())
} 