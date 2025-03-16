//! IPv6 Overlay Network Example
//!
//! This example demonstrates how to create an IPv6-based overlay network
//! for ICN nodes, with tunneling, routing, and peer discovery.

use std::error::Error;
use std::net::{Ipv6Addr, SocketAddr, IpAddr};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, debug, error, Level};
use tracing_subscriber::FmtSubscriber;

use icn_network::{
    networking::{
        overlay::{
            OverlayNetworkManager, OverlayNetworkService, OverlayAddress,
            OverlayOptions, MessagePriority, TunnelType, ForwardingPolicy,
            address::{AddressAllocator, AddressSpace, AddressAllocationStrategy},
            tunneling::TunnelManager,
        },
        Result,
    },
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    info!("Starting IPv6 overlay network example");
    
    // Create an address allocator
    let mut address_allocator = AddressAllocator::with_settings(
        AddressSpace::UniqueLocal,
        AddressAllocationStrategy::FederationPrefixed,
        48,  // Federation prefix length
        64   // Node prefix length
    );
    
    // Create overlay managers for nodes in different federations
    let mut node1 = OverlayNetworkManager::with_address_allocator(address_allocator.clone());
    let mut node2 = OverlayNetworkManager::with_address_allocator(address_allocator.clone());
    let mut node3 = OverlayNetworkManager::with_address_allocator(address_allocator.clone());
    
    // Initialize nodes with different federation IDs
    let federation1 = "federation-alpha";
    let federation2 = "federation-beta";
    
    // Initialize the nodes
    info!("Initializing node 1 (federation: {})", federation1);
    let addr1 = node1.initialize("node1", Some(federation1)).await?;
    
    info!("Initializing node 2 (federation: {})", federation1);
    let addr2 = node2.initialize("node2", Some(federation1)).await?;
    
    info!("Initializing node 3 (federation: {})", federation2);
    let addr3 = node3.initialize("node3", Some(federation2)).await?;
    
    info!("Node addresses:");
    info!("  Node 1: {}", addr1);
    info!("  Node 2: {}", addr2);
    info!("  Node 3: {}", addr3);
    
    // Connect nodes (node1 is the bootstrap node)
    info!("Connecting nodes to the overlay network");
    
    // Node 2 connects to node 1 (same federation)
    node2.connect(&[addr1.clone()]).await?;
    
    // Node 3 connects to node 1 (different federation)
    node3.connect(&[addr1.clone()]).await?;
    
    // Set forwarding policies
    node1.set_forwarding_policy(ForwardingPolicy::ForwardAll)?;
    node2.set_forwarding_policy(ForwardingPolicy::ForwardKnown)?;
    node3.set_forwarding_policy(ForwardingPolicy::ForwardKnown)?;
    
    // Create tunnels between nodes
    info!("Creating tunnels between nodes");
    
    // Direct tunnel for nodes in the same federation
    if let Some(local_addr1) = node1.get_local_address() {
        let tunnel12 = node1.create_tunnel(&addr2, TunnelType::Direct).await?;
        info!("Created direct tunnel from node 1 to node 2: {}", tunnel12.id);
    }
    
    // WireGuard tunnel for nodes in different federations
    if let Some(local_addr1) = node1.get_local_address() {
        let tunnel13 = node1.create_tunnel(&addr3, TunnelType::WireGuard).await?;
        info!("Created WireGuard tunnel from node 1 to node 3: {}", tunnel13.id);
    }
    
    // Show active tunnels for node 1
    let tunnels = node1.get_tunnels()?;
    info!("Node 1 active tunnels: {}", tunnels.len());
    for tunnel in tunnels {
        info!("  Tunnel ID: {}", tunnel.id);
        info!("  Type: {:?}", tunnel.tunnel_type);
        info!("  Remote: {}", tunnel.remote_overlay_addr);
        info!("  Active: {}", tunnel.active);
    }
    
    // Send data from node 2 to node 3 (via node 1)
    info!("Sending data from node 2 to node 3 (via node 1)");
    let data = b"Hello from federation Alpha to federation Beta!";
    let options = OverlayOptions {
        anonymity_required: false,
        reliability_required: true,
        priority: MessagePriority::Normal,
        tunnel_type: Some(TunnelType::WireGuard),
        ttl: 64,
    };
    
    // Send data
    node2.send_data(&addr3, data, &options).await?;
    
    // In a real implementation, we would wait for and handle received data
    // Simulate that by waiting a bit
    info!("Waiting for transmission...");
    sleep(Duration::from_secs(1)).await;
    
    info!("Example completed successfully");
    Ok(())
} 