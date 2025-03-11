use icn_networking::{
    node::{Node, NodeConfig, NodeType},
    tls::TlsConfig,
    test_utils::generate_test_certificate,
};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    time::Duration,
    collections::HashMap,
};
use tokio;

const NUM_COOPS: usize = 3;
const NODES_PER_COOP: usize = 2;
const BASE_PORT: u16 = 9000;

#[tokio::main]
async fn main() {
    // Generate certificates for nodes
    let (cert_chain, private_key) = generate_test_certificate();
    
    // Create TLS config that will be shared by all nodes
    let tls_config = TlsConfig::new(
        cert_chain.clone(),
        private_key.clone_key(),
        Some(cert_chain.clone()),
    ).expect("Failed to create TLS config");

    // Track nodes by cooperative
    let mut coop_nodes: HashMap<String, Vec<NodeConfig>> = HashMap::new();
    let mut port_counter = BASE_PORT;

    // Create nodes for each cooperative
    for coop_id in 0..NUM_COOPS {
        let coop_name = format!("coop-{}", coop_id);
        let mut nodes = Vec::new();

        // Create primary and secondary nodes for each cooperative
        for node_idx in 0..NODES_PER_COOP {
            let node_type = if node_idx == 0 { NodeType::Primary } else { NodeType::Secondary };
            let node_port = port_counter;
            port_counter += 1;

            // Collect peer addresses from other cooperatives' primary nodes
            let mut peers = Vec::new();
            for (_, other_nodes) in &coop_nodes {
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
                node_id: format!("{}-node-{}", coop_name, node_idx),
                coop_id: coop_name.clone(),
                node_type: node_type.clone(),
                discovery_interval: Some(Duration::from_secs(30)),
                health_check_interval: Some(Duration::from_secs(10)),
            };

            let mut node = Node::new(config.clone(), tls_config.clone());
            nodes.push(config.clone());
            
            // Start node in background
            let node_id = format!("{}-node-{}", coop_name, node_idx);
            tokio::spawn(async move {
                if let Err(e) = node.start().await {
                    eprintln!("Node {} failed: {}", node_id, e);
                }
            });
        }

        coop_nodes.insert(coop_name, nodes);
    }

    // Print network topology
    println!("\nNetwork Topology:");
    println!("================");
    for (coop_id, nodes) in &coop_nodes {
        println!("\nCooperative: {}", coop_id);
        for node in nodes {
            println!("  {} ({}): {}", node.node_id, node.node_type, node.listen_addr);
            if !node.peers.is_empty() {
                println!("    Peers: {:?}", node.peers);
            }
        }
    }

    println!("\nTestnet is running. Press Ctrl+C to stop.");

    // Keep the main thread running
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    }
} 