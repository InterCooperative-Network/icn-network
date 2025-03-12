use std::error::Error;
use std::net::SocketAddr;
use std::time::Duration;
use std::sync::Arc;
use tokio::time;
use std::collections::HashMap;

use icn_networking::node::{Node, NodeConfig, NodeType, NetworkService};
use icn_networking::tls::TlsConfig;
use icn_did::manager::{DidManager, DidManagerConfig, CreateDidOptions};
use icn_did::resolver::{ResolutionResult, DidResolver};
use icn_did::federation::FederationClient;
use icn_crypto::{KeyPair, KeyType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::fmt::init();
    
    println!("Starting ICN Identity and Networking Integration Example");
    
    // Setup two federated nodes
    let fed1_node_addr: SocketAddr = "127.0.0.1:9100".parse()?;
    let fed2_node_addr: SocketAddr = "127.0.0.1:9200".parse()?;
    
    // Create TLS configuration
    let tls_config = TlsConfig::default();
    
    // Setup federation 1
    println!("\nSetting up Federation 1");
    let fed1_config = NodeConfig {
        listen_addr: fed1_node_addr,
        peers: vec![fed2_node_addr],
        node_id: "fed1-node".to_string(),
        coop_id: "coop-fed1".to_string(),
        node_type: NodeType::Primary,
        discovery_interval: Some(Duration::from_secs(5)),
        health_check_interval: Some(Duration::from_secs(10)),
    };
    
    let mut fed1_node = Node::new(
        "fed1-node".to_string(), 
        fed1_node_addr, 
        tls_config.clone(),
        fed1_config.clone(),
    );
    
    // Setup federation 2
    println!("Setting up Federation 2");
    let fed2_config = NodeConfig {
        listen_addr: fed2_node_addr,
        peers: vec![fed1_node_addr],
        node_id: "fed2-node".to_string(),
        coop_id: "coop-fed2".to_string(),
        node_type: NodeType::Primary,
        discovery_interval: Some(Duration::from_secs(5)),
        health_check_interval: Some(Duration::from_secs(10)),
    };
    
    let mut fed2_node = Node::new(
        "fed2-node".to_string(), 
        fed2_node_addr, 
        tls_config.clone(),
        fed2_config.clone(),
    );
    
    // Start the network nodes
    println!("Starting network nodes");
    fed1_node.start().await?;
    fed2_node.start().await?;
    
    // Announce federations
    println!("Announcing federations");
    fed1_node.announce_federation(
        "federation1".to_string(),
        "First federation for example".to_string(),
        vec![fed1_node_addr],
        vec!["identity".to_string(), "mutual-credit".to_string()],
    ).await?;
    
    fed2_node.announce_federation(
        "federation2".to_string(),
        "Second federation for example".to_string(),
        vec![fed2_node_addr],
        vec!["identity".to_string(), "governance".to_string()],
    ).await?;
    
    // Allow discovery to happen
    println!("Waiting for discovery...");
    time::sleep(Duration::from_secs(2)).await;
    
    // Setup DID managers for both federations
    println!("\nSetting up DID managers");
    let fed1_did_config = DidManagerConfig {
        storage_options: Default::default(),
        default_key_type: KeyType::Ed25519,
        challenge_ttl_seconds: 3600,
        federation_id: "federation1".to_string(),
        federation_endpoints: vec![format!("http://{}", fed1_node_addr)],
        retain_private_keys: true,
    };
    
    let fed2_did_config = DidManagerConfig {
        storage_options: Default::default(),
        default_key_type: KeyType::Ed25519,
        challenge_ttl_seconds: 3600,
        federation_id: "federation2".to_string(),
        federation_endpoints: vec![format!("http://{}", fed2_node_addr)],
        retain_private_keys: true,
    };
    
    // Create federation clients
    let mut federation_endpoints = HashMap::new();
    federation_endpoints.insert("federation1".to_string(), format!("http://{}", fed1_node_addr));
    federation_endpoints.insert("federation2".to_string(), format!("http://{}", fed2_node_addr));
    let federation_client = Arc::new(FederationClient::new(federation_endpoints));
    
    // Create DID managers for each federation
    let fed1_did_manager = DidManager::new(fed1_did_config).await?;
    let fed2_did_manager = DidManager::new(fed2_did_config).await?;
    
    // Create DIDs in each federation
    println!("Creating DIDs in each federation");
    let (fed1_did_doc, fed1_keypair) = fed1_did_manager.create_did(CreateDidOptions::default()).await?;
    println!("Federation 1 DID: {}", fed1_did_doc.id);
    
    let (fed2_did_doc, fed2_keypair) = fed2_did_manager.create_did(CreateDidOptions::default()).await?;
    println!("Federation 2 DID: {}", fed2_did_doc.id);
    
    // Store DIDs in resolver
    fed1_did_manager.store(&fed1_did_doc.id, fed1_did_doc.clone()).await?;
    fed2_did_manager.store(&fed2_did_doc.id, fed2_did_doc.clone()).await?;
    
    // Attempt cross-federation DID resolution
    println!("\nTesting cross-federation DID resolution");
    
    // Resolve within the same federation (local)
    println!("Resolving DID within same federation (local)");
    let local_resolution = fed1_did_manager.resolve(&fed1_did_doc.id).await?;
    match local_resolution.document {
        Some(doc) => println!("Successfully resolved local DID: {}", doc.id),
        None => println!("Failed to resolve local DID"),
    }
    
    // Simulate cross-federation resolution
    println!("Simulating cross-federation resolution");
    println!("Federation 1 resolving DID from Federation 2: {}", fed2_did_doc.id);
    
    // In a real implementation, this would connect to the other federation's network
    // For this example, we'll directly use the resolver
    let fed1_resolver = fed1_did_manager.resolver();
    let fed2_resolver = fed2_did_manager.resolver();
    
    // Let's simulate federation interaction by reading the DID from fed2 and making it available to fed1
    let fed2_did_result = fed2_resolver.resolve(&fed2_did_doc.id).await?;
    if let Some(doc) = fed2_did_result.document {
        println!("Federation 2 provided DID document for: {}", doc.id);
        
        // Now simulate fed1 storing this DID from fed2
        fed1_resolver.store(&doc.id, doc.clone()).await?;
        
        // Now fed1 should be able to resolve the fed2 DID
        let cross_fed_resolution = fed1_resolver.resolve(&fed2_did_doc.id).await?;
        match cross_fed_resolution.document {
            Some(doc) => println!("Federation 1 successfully resolved Federation 2 DID: {}", doc.id),
            None => println!("Federation 1 failed to resolve Federation 2 DID"),
        }
    } else {
        println!("Could not retrieve DID document from Federation 2");
    }
    
    // Demonstrate signature verification across federations
    println!("\nTesting cross-federation signature verification");
    
    // Create a message and sign it with fed2's private key
    let message = b"This is a test message from Federation 2";
    let signature = fed2_keypair.sign(message)?;
    
    // Verify the signature using fed1's resolver
    println!("Verifying Federation 2 signature in Federation 1");
    let verification_result = fed1_did_manager.verify_signature(
        &fed2_did_doc.id,
        &fed2_did_doc.verification_method[0].id,
        message,
        &signature,
    ).await?;
    
    if verification_result {
        println!("Signature verification successful across federations");
    } else {
        println!("Signature verification failed across federations");
    }
    
    println!("\nExample completed successfully");
    
    Ok(())
} 