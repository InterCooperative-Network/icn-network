use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use tokio::time;

use icn_networking::node::{Node, NodeConfig, NodeType, NetworkService};
use icn_networking::tls::TlsConfig;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();
    
    // Parse socket addresses for our nodes
    let main_node_addr: SocketAddr = "127.0.0.1:9000".parse()?;
    let peer_addr: SocketAddr = "127.0.0.1:9001".parse()?;
    
    println!("Starting ICN node example...");
    
    // Setup main node configuration
    let main_config = NodeConfig {
        listen_addr: main_node_addr,
        peers: vec![peer_addr],  // Configure initial peer
        node_id: "main-node".to_string(),
        coop_id: "example-coop".to_string(),
        node_type: NodeType::Primary,
        discovery_interval: Some(Duration::from_secs(5)),
        health_check_interval: Some(Duration::from_secs(10)),
    };
    
    // Create a TLS configuration
    // In a real application, you'd use certificates from a file
    let tls_config = TlsConfig::default();
    
    // Create the main node
    let mut main_node = Node::new(
        "main-node".to_string(),
        main_node_addr,
        tls_config.clone(),
        main_config,
    );
    
    // Setup peer node configuration
    let peer_config = NodeConfig {
        listen_addr: peer_addr,
        peers: vec![main_node_addr],  // Configure main node as peer
        node_id: "peer-node".to_string(),
        coop_id: "example-coop".to_string(),
        node_type: NodeType::Secondary,
        discovery_interval: Some(Duration::from_secs(5)),
        health_check_interval: Some(Duration::from_secs(10)),
    };
    
    // Create the peer node
    let mut peer_node = Node::new(
        "peer-node".to_string(),
        peer_addr,
        tls_config.clone(),
        peer_config,
    );
    
    // Start both nodes
    println!("Starting main node...");
    main_node.start().await?;
    
    println!("Starting peer node...");
    peer_node.start().await?;
    
    // Announce a federation to the network
    println!("Announcing federation...");
    main_node.announce_federation(
        "example-federation".to_string(),
        "Example federation for demo".to_string(),
        vec![main_node_addr],
        vec!["identity".to_string(), "mutual-credit".to_string()],
    ).await?;
    
    // Wait for discovery to happen
    println!("Waiting for discovery...");
    time::sleep(Duration::from_secs(10)).await;
    
    // Check peers
    let main_peers = main_node.get_peers()?;
    println!("Main node peers: {}", main_peers.len());
    for peer in &main_peers {
        println!("  Peer: {}, {}", peer.id, peer.address);
    }
    
    let peer_peers = peer_node.get_peers()?;
    println!("Peer node peers: {}", peer_peers.len());
    for peer in &peer_peers {
        println!("  Peer: {}, {}", peer.id, peer.address);
    }
    
    // Check federations
    let federations = main_node.get_federations()?;
    println!("Known federations: {}", federations.len());
    for federation in &federations {
        println!("  Federation: {}", federation.federation_id);
        println!("    Description: {}", federation.description);
        println!("    Services: {:?}", federation.services);
    }
    
    // Keep running for demonstration
    println!("Nodes are running. Press Ctrl+C to exit.");
    loop {
        time::sleep(Duration::from_secs(1)).await;
    }
} 