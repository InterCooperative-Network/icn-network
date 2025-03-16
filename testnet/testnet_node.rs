//! ICN IPv6 Overlay Network Testnet Node
//!
//! This implements a single node in the testnet that can be run
//! as part of testing the overlay network implementation.

use std::error::Error;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

use icn_network::{
    networking::{
        overlay::{
            OverlayNetworkManager, OverlayNetworkService, OverlayAddress, 
            OverlayOptions, MessagePriority, TunnelType, ForwardingPolicy,
            address::{AddressAllocator, AddressSpace, AddressAllocationStrategy},
            tunneling::TunnelManager,
        },
        Result, NetworkError,
    },
};

/// Node statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct NodeStats {
    /// Number of peers connected
    peers: usize,
    /// Number of active tunnels
    tunnels: usize,
    /// Messages sent
    messages_sent: u64,
    /// Messages received
    messages_received: u64,
    /// Bytes sent
    bytes_sent: u64,
    /// Bytes received
    bytes_received: u64,
    /// Uptime in seconds
    uptime_seconds: u64,
}

/// Testnet node wrapper
struct TestnetNode {
    /// Node ID
    node_id: String,
    /// Federation ID
    federation_id: String,
    /// Overlay network manager
    overlay: OverlayNetworkManager,
    /// Local overlay address
    local_address: Option<OverlayAddress>,
    /// Node statistics
    stats: Arc<Mutex<NodeStats>>,
    /// Running flag
    running: Arc<Mutex<bool>>,
}

impl TestnetNode {
    /// Create a new testnet node
    pub fn new(node_id: &str, federation_id: &str, address_space: AddressSpace, forwarding_policy: ForwardingPolicy) -> Self {
        // Create address allocator
        let mut address_allocator = AddressAllocator::with_settings(
            address_space,
            AddressAllocationStrategy::FederationPrefixed,
            48,  // Federation prefix length
            64   // Node prefix length
        );
        
        // Create overlay network manager
        let overlay = OverlayNetworkManager::with_address_allocator(address_allocator);
        
        Self {
            node_id: node_id.to_string(),
            federation_id: federation_id.to_string(),
            overlay,
            local_address: None,
            stats: Arc::new(Mutex::new(NodeStats::default())),
            running: Arc::new(Mutex::new(false)),
        }
    }
    
    /// Initialize the node
    pub async fn initialize(&mut self) -> Result<OverlayAddress> {
        info!("Initializing node: {}", self.node_id);
        info!("Federation: {}", self.federation_id);
        
        // Initialize the overlay network
        let federation_id = if self.federation_id.is_empty() {
            None
        } else {
            Some(self.federation_id.as_str())
        };
        
        let address = self.overlay.initialize(&self.node_id, federation_id).await?;
        self.local_address = Some(address.clone());
        
        info!("Node initialized with overlay address: {}", address);
        
        Ok(address)
    }
    
    /// Connect to bootstrap peers
    pub async fn connect_to_bootstrap(&mut self, bootstrap_peers: &[String]) -> Result<()> {
        if bootstrap_peers.is_empty() {
            info!("No bootstrap peers specified, running in standalone mode");
            return Ok(());
        }
        
        info!("Connecting to bootstrap peers: {:?}", bootstrap_peers);
        
        // Parse bootstrap peer addresses into OverlayAddress objects
        let mut overlay_addresses = Vec::new();
        
        for peer_addr in bootstrap_peers {
            match peer_addr.parse::<OverlayAddress>() {
                Ok(addr) => {
                    overlay_addresses.push(addr);
                },
                Err(e) => {
                    warn!("Failed to parse bootstrap peer address: {}: {}", peer_addr, e);
                }
            }
        }
        
        if overlay_addresses.is_empty() {
            warn!("No valid bootstrap peer addresses found");
            return Ok(());
        }
        
        // Connect to the bootstrap peers
        self.overlay.connect(&overlay_addresses).await?;
        
        // Update peer count in stats
        let peers = self.overlay.get_peers()?;
        let mut stats = self.stats.lock().unwrap();
        stats.peers = peers.len();
        
        info!("Connected to {} peers", peers.len());
        
        Ok(())
    }
    
    /// Run the node
    pub async fn run(&mut self) -> Result<()> {
        {
            let mut running = self.running.lock().unwrap();
            *running = true;
        }
        
        // Start status updater
        self.start_status_updater();
        
        // Start message receiver
        self.start_message_receiver();
        
        // Main loop
        info!("Node {} running", self.node_id);
        
        while *self.running.lock().unwrap() {
            // Sleep to avoid busy-waiting
            sleep(Duration::from_secs(1)).await;
        }
        
        info!("Node shutting down");
        Ok(())
    }
    
    /// Start background task to update node status
    fn start_status_updater(&self) {
        let node_id = self.node_id.clone();
        let overlay = self.overlay.clone();
        let stats = Arc::clone(&self.stats);
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            let start_time = std::time::Instant::now();
            
            while *running.lock().unwrap() {
                // Update stats
                let mut stats_guard = stats.lock().unwrap();
                
                // Update uptime
                stats_guard.uptime_seconds = start_time.elapsed().as_secs();
                
                // Update peer count
                match overlay.get_peers() {
                    Ok(peers) => {
                        stats_guard.peers = peers.len();
                    },
                    Err(e) => {
                        error!("Failed to get peers: {}", e);
                    }
                }
                
                // Update tunnel count
                match overlay.get_tunnels() {
                    Ok(tunnels) => {
                        stats_guard.tunnels = tunnels.len();
                    },
                    Err(e) => {
                        error!("Failed to get tunnels: {}", e);
                    }
                }
                
                drop(stats_guard);
                
                // Print current status
                if let Ok(stats) = stats.lock() {
                    info!(
                        "Node {} | Uptime: {}s | Peers: {} | Tunnels: {} | Msgs: Sent={}, Recv={} | Bytes: Sent={}, Recv={}",
                        node_id,
                        stats.uptime_seconds,
                        stats.peers,
                        stats.tunnels,
                        stats.messages_sent,
                        stats.messages_received,
                        stats.bytes_sent,
                        stats.bytes_received
                    );
                }
                
                // Update every 10 seconds
                sleep(Duration::from_secs(10)).await;
            }
        });
    }
    
    /// Start message receiver task
    fn start_message_receiver(&self) {
        let node_id = self.node_id.clone();
        let overlay = self.overlay.clone();
        let stats = Arc::clone(&self.stats);
        let running = Arc::clone(&self.running);
        
        tokio::spawn(async move {
            while *running.lock().unwrap() {
                // Try to receive data from the overlay
                match overlay.receive_data().await {
                    Ok((source_addr, data)) => {
                        let data_len = data.len();
                        
                        // Try to interpret as a string message
                        let message = String::from_utf8_lossy(&data);
                        info!("Received message from {}: {}", source_addr, message);
                        
                        // Update stats
                        let mut stats_guard = stats.lock().unwrap();
                        stats_guard.messages_received += 1;
                        stats_guard.bytes_received += data_len as u64;
                        
                        // If it's a ping message, send a pong response
                        if message.contains("PING") {
                            drop(stats_guard);
                            
                            // Sleep a bit to simulate processing
                            sleep(Duration::from_millis(100)).await;
                            
                            // Send pong response
                            let response = format!("PONG from node {}", node_id);
                            let options = OverlayOptions::default();
                            
                            match overlay.send_data(&source_addr, response.as_bytes(), &options).await {
                                Ok(_) => {
                                    info!("Sent PONG response to {}", source_addr);
                                    
                                    // Update stats
                                    let mut stats_guard = stats.lock().unwrap();
                                    stats_guard.messages_sent += 1;
                                    stats_guard.bytes_sent += response.len() as u64;
                                },
                                Err(e) => {
                                    error!("Failed to send PONG response: {}", e);
                                }
                            }
                        }
                    },
                    Err(e) => {
                        // Ignore "no data available" errors as they're expected
                        if let NetworkError::Other(msg) = &e {
                            if msg != "No data available" && !msg.contains("Packet forwarded") {
                                error!("Error receiving data: {}", e);
                            }
                        } else {
                            error!("Error receiving data: {}", e);
                        }
                    }
                }
                
                // Small sleep to avoid busy-waiting
                sleep(Duration::from_millis(100)).await;
            }
        });
    }
    
    /// Send a ping message to a destination
    pub async fn send_ping(&self, destination: &OverlayAddress) -> Result<()> {
        let message = format!("PING from node {}", self.node_id);
        info!("Sending ping to {}: {}", destination, message);
        
        let options = OverlayOptions {
            anonymity_required: false,
            reliability_required: true,
            priority: MessagePriority::Normal,
            tunnel_type: None, // Use default tunnel type
            ttl: 64,
        };
        
        // Send the message
        self.overlay.send_data(destination, message.as_bytes(), &options).await?;
        
        // Update stats
        let mut stats = self.stats.lock().unwrap();
        stats.messages_sent += 1;
        stats.bytes_sent += message.len() as u64;
        
        Ok(())
    }
    
    /// Shutdown the node
    pub fn shutdown(&self) {
        let mut running = self.running.lock().unwrap();
        *running = false;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let matches = App::new("ICN IPv6 Overlay Network Testnet Node")
        .version("0.1.0")
        .author("ICN Network Team")
        .about("Runs a node in the ICN IPv6 overlay network testnet")
        .arg(Arg::with_name("node-id")
            .long("node-id")
            .value_name("ID")
            .help("Unique node identifier")
            .takes_value(true)
            .required(true))
        .arg(Arg::with_name("federation")
            .long("federation")
            .value_name("ID")
            .help("Federation identifier (empty for no federation)")
            .takes_value(true)
            .default_value(""))
        .arg(Arg::with_name("port")
            .long("port")
            .value_name("PORT")
            .help("Port to listen on")
            .takes_value(true)
            .default_value("9000"))
        .arg(Arg::with_name("bootstrap-peers")
            .long("bootstrap-peers")
            .value_name("PEERS")
            .help("Comma-separated list of bootstrap peer addresses")
            .takes_value(true)
            .default_value(""))
        .arg(Arg::with_name("forwarding-policy")
            .long("forwarding-policy")
            .value_name("POLICY")
            .help("Packet forwarding policy (ForwardAll, ForwardKnown, NoForwarding)")
            .takes_value(true)
            .default_value("ForwardKnown"))
        .arg(Arg::with_name("log-level")
            .long("log-level")
            .value_name("LEVEL")
            .help("Log level (trace, debug, info, warn, error)")
            .takes_value(true)
            .default_value("info"))
        .get_matches();
    
    // Get arguments
    let node_id = matches.value_of("node-id").unwrap().to_string();
    let federation_id = matches.value_of("federation").unwrap().to_string();
    let _port = matches.value_of("port").unwrap().parse::<u16>()?;
    
    let bootstrap_peers = matches.value_of("bootstrap-peers").unwrap().to_string();
    let bootstrap_peers: Vec<String> = if bootstrap_peers.is_empty() {
        Vec::new()
    } else {
        bootstrap_peers.split(',').map(|s| s.trim().to_string()).collect()
    };
    
    let forwarding_policy = match matches.value_of("forwarding-policy").unwrap() {
        "ForwardAll" => ForwardingPolicy::ForwardAll,
        "ForwardKnown" => ForwardingPolicy::ForwardKnown,
        "NoForwarding" => ForwardingPolicy::NoForwarding,
        _ => ForwardingPolicy::ForwardKnown,
    };
    
    let log_level = match matches.value_of("log-level").unwrap() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    // Create and initialize node
    let mut node = TestnetNode::new(&node_id, &federation_id, AddressSpace::UniqueLocal, forwarding_policy);
    node.initialize().await?;
    
    // Connect to bootstrap peers
    node.connect_to_bootstrap(&bootstrap_peers).await?;
    
    // Run the node
    node.run().await?;
    
    Ok(())
} 