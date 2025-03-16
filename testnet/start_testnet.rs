//! ICN IPv6 Overlay Network Testnet Starter
//!
//! This script reads a testnet configuration file and launches nodes
//! according to the specifications.

use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::{App, Arg};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tracing::{debug, error, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Main testnet configuration
#[derive(Debug, Deserialize)]
struct TestnetConfig {
    testnet: TestnetInfo,
    network: NetworkConfig,
    federations: HashMap<String, FederationConfig>,
    nodes: HashMap<String, NodeConfig>,
    simulation: Option<SimulationConfig>,
}

#[derive(Debug, Deserialize)]
struct TestnetInfo {
    name: String,
    description: String,
    version: String,
}

#[derive(Debug, Deserialize)]
struct NetworkConfig {
    bootstrap_nodes: Vec<String>,
    min_peers: usize,
    max_peers: usize,
    connection_timeout_ms: u64,
    peer_discovery_interval_ms: u64,
    overlay: OverlayConfig,
}

#[derive(Debug, Deserialize)]
struct OverlayConfig {
    address_space: String,
    allocation_strategy: String,
    federation_prefix_len: u8,
    node_prefix_len: u8,
    default_tunnel_type: String,
}

#[derive(Debug, Deserialize)]
struct FederationConfig {
    name: String,
    description: String,
    bootstrap_nodes: Vec<String>,
    min_nodes: usize,
    forwarding_policy: String,
}

#[derive(Debug, Deserialize)]
struct NodeConfig {
    name: String,
    federation: String,
    role: String,
    listen_port: u16,
    forwarding_policy: String,
    log_level: String,
}

#[derive(Debug, Deserialize)]
struct SimulationConfig {
    enabled: bool,
    duration_seconds: u64,
    message_interval_ms: u64,
    failure_probability: f64,
    latency_min_ms: u64,
    latency_max_ms: u64,
}

/// Node process handle and info
struct NodeHandle {
    node_id: String,
    process: Child,
    listen_port: u16,
    federation: String,
    role: String,
}

impl Drop for NodeHandle {
    fn drop(&mut self) {
        // Try to gracefully shut down node when handle is dropped
        match self.process.kill() {
            Ok(_) => info!("Node {} process terminated", self.node_id),
            Err(e) => error!("Failed to terminate node {} process: {}", self.node_id, e),
        }
    }
}

/// Testnet manager
struct TestnetManager {
    config: TestnetConfig,
    nodes: Arc<Mutex<HashMap<String, NodeHandle>>>,
    node_addresses: Arc<Mutex<HashMap<String, String>>>, // Node ID -> IPv6 address
}

impl TestnetManager {
    /// Create a new testnet manager from a config file
    fn new(config_path: &str) -> Result<Self, Box<dyn Error>> {
        let mut file = File::open(config_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        
        let config: TestnetConfig = toml::from_str(&contents)?;
        
        Ok(Self {
            config,
            nodes: Arc::new(Mutex::new(HashMap::new())),
            node_addresses: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    /// Start the testnet nodes
    async fn start_testnet(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting testnet: {}", self.config.testnet.name);
        info!("Description: {}", self.config.testnet.description);
        
        // Start bootstrap nodes first
        for bootstrap_node_id in &self.config.network.bootstrap_nodes {
            if let Some(node_config) = self.config.nodes.get(bootstrap_node_id) {
                info!("Starting bootstrap node: {}", bootstrap_node_id);
                self.start_node(bootstrap_node_id, node_config).await?;
                // Give bootstrap nodes time to initialize
                sleep(Duration::from_secs(2)).await;
            } else {
                warn!("Bootstrap node {} not found in configuration", bootstrap_node_id);
            }
        }
        
        // Start federation bootstrap nodes if not already started
        for (fed_id, fed_config) in &self.config.federations {
            for node_id in &fed_config.bootstrap_nodes {
                if !self.config.network.bootstrap_nodes.contains(node_id) {
                    if let Some(node_config) = self.config.nodes.get(node_id) {
                        info!("Starting federation {} bootstrap node: {}", fed_id, node_id);
                        self.start_node(node_id, node_config).await?;
                        sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        }
        
        // Start remaining nodes
        for (node_id, node_config) in &self.config.nodes {
            if !self.config.network.bootstrap_nodes.contains(node_id) {
                let federation = &node_config.federation;
                let is_fed_bootstrap = self.config.federations.get(federation)
                    .map_or(false, |f| f.bootstrap_nodes.contains(node_id));
                
                if !is_fed_bootstrap {
                    info!("Starting node: {}", node_id);
                    self.start_node(node_id, node_config).await?;
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
        
        info!("All nodes started");
        self.print_testnet_status().await;
        
        Ok(())
    }
    
    /// Start a single node
    async fn start_node(&self, node_id: &str, node_config: &NodeConfig) -> Result<(), Box<dyn Error>> {
        // Generate arguments for the node process
        let federation_id = &node_config.federation;
        let listen_port = node_config.listen_port;
        
        // Determine bootstrap peers to connect to
        let bootstrap_peers = if node_config.role == "bootstrap" {
            // Bootstrap nodes don't connect to other bootstrap nodes initially
            Vec::new()
        } else {
            // Regular nodes connect to federation bootstrap nodes
            let federation = self.config.federations.get(federation_id)
                .ok_or_else(|| format!("Federation {} not found", federation_id))?;
            
            let mut peers = Vec::new();
            let node_addresses = self.node_addresses.lock().unwrap();
            
            for bootstrap_id in &federation.bootstrap_nodes {
                if let Some(addr) = node_addresses.get(bootstrap_id) {
                    peers.push(addr.clone());
                }
            }
            
            // If no federation bootstrap nodes are available, use global bootstrap nodes
            if peers.is_empty() {
                for bootstrap_id in &self.config.network.bootstrap_nodes {
                    if let Some(addr) = node_addresses.get(bootstrap_id) {
                        peers.push(addr.clone());
                    }
                }
            }
            
            peers
        };
        
        // Build the command to run a node (simulated as a process)
        let cmd_args = format!(
            "--node-id {} --federation {} --port {} --forwarding-policy {} --log-level {}{}",
            node_id,
            federation_id,
            listen_port,
            node_config.forwarding_policy,
            node_config.log_level,
            if bootstrap_peers.is_empty() {
                "".to_string()
            } else {
                format!(" --bootstrap-peers {}", bootstrap_peers.join(","))
            }
        );
        
        info!("Starting node with args: {}", cmd_args);
        
        // In a real implementation, this would launch the actual node process
        // For simulation, we'll just log that we would start it
        let process = Command::new("echo")
            .arg(format!("Node {} would start with: {}", node_id, cmd_args))
            .spawn()?;
        
        // Create node handle
        let node_handle = NodeHandle {
            node_id: node_id.to_string(),
            process,
            listen_port,
            federation: federation_id.to_string(),
            role: node_config.role.clone(),
        };
        
        // Generate a simulated IPv6 address for the node
        let ipv6_addr = format!("fd00:dead:beef:{}::{}/64", node_handle.federation.as_bytes()[0], node_handle.listen_port);
        
        // Store node handle and address
        {
            let mut nodes = self.nodes.lock().unwrap();
            nodes.insert(node_id.to_string(), node_handle);
            
            let mut addresses = self.node_addresses.lock().unwrap();
            addresses.insert(node_id.to_string(), ipv6_addr.clone());
        }
        
        info!("Node {} started with overlay address: {}", node_id, ipv6_addr);
        
        Ok(())
    }
    
    /// Print status of the testnet
    async fn print_testnet_status(&self) {
        info!("╔═════════════════════════════════════════════════════════════════════════════╗");
        info!("║                              TESTNET STATUS                                  ║");
        info!("╠═════════════════════════════════════════════════════════════════════════════╣");
        info!("║ Name: {:72} ║", self.config.testnet.name);
        info!("║ Version: {:69} ║", self.config.testnet.version);
        info!("╠═════════════════════════════════════════════════════════════════════════════╣");
        info!("║ NODES                                                                       ║");
        info!("╠═════════════════════════════════════════════════════════════════════════════╣");
        
        let nodes = self.nodes.lock().unwrap();
        let addresses = self.node_addresses.lock().unwrap();
        
        for (node_id, node) in nodes.iter() {
            let addr = addresses.get(node_id).unwrap_or(&"unknown".to_string());
            info!("║ {:10} | Federation: {:10} | Role: {:10} | Address: {:22} ║", 
                node_id, node.federation, node.role, addr);
        }
        
        info!("╠═════════════════════════════════════════════════════════════════════════════╣");
        info!("║ FEDERATIONS                                                                 ║");
        info!("╠═════════════════════════════════════════════════════════════════════════════╣");
        
        for (fed_id, fed_config) in &self.config.federations {
            let fed_nodes: Vec<_> = nodes.iter()
                .filter(|(_, n)| n.federation == *fed_id)
                .map(|(id, _)| id.clone())
                .collect();
            
            info!("║ {:10} | Nodes: {:3} | Bootstrap: {:35} ║", 
                fed_id, 
                fed_nodes.len(),
                fed_config.bootstrap_nodes.join(", "));
        }
        
        info!("╚═════════════════════════════════════════════════════════════════════════════╝");
    }
    
    /// Run simulation if enabled
    async fn run_simulation(&self) -> Result<(), Box<dyn Error>> {
        if let Some(sim_config) = &self.config.simulation {
            if sim_config.enabled {
                info!("Starting testnet simulation for {} seconds", sim_config.duration_seconds);
                
                // Run simulation for specified duration
                let end_time = std::time::Instant::now() + Duration::from_secs(sim_config.duration_seconds);
                
                while std::time::Instant::now() < end_time {
                    // Simulate random communication between nodes
                    self.simulate_node_communication().await?;
                    
                    // Randomly introduce node failures if configured
                    if rand::random::<f64>() < sim_config.failure_probability {
                        self.simulate_node_failure().await?;
                    }
                    
                    // Wait for next simulation cycle
                    sleep(Duration::from_millis(sim_config.message_interval_ms)).await;
                }
                
                info!("Testnet simulation completed");
            }
        }
        
        Ok(())
    }
    
    /// Simulate communication between random nodes
    async fn simulate_node_communication(&self) -> Result<(), Box<dyn Error>> {
        let nodes = self.nodes.lock().unwrap();
        let addresses = self.node_addresses.lock().unwrap();
        
        if nodes.len() < 2 {
            return Ok(());
        }
        
        // Select random source and destination nodes
        let node_ids: Vec<String> = nodes.keys().cloned().collect();
        let src_idx = rand::random::<usize>() % node_ids.len();
        let mut dst_idx = rand::random::<usize>() % node_ids.len();
        while dst_idx == src_idx {
            dst_idx = rand::random::<usize>() % node_ids.len();
        }
        
        let src_id = &node_ids[src_idx];
        let dst_id = &node_ids[dst_idx];
        
        let src_addr = addresses.get(src_id).cloned().unwrap_or_default();
        let dst_addr = addresses.get(dst_id).cloned().unwrap_or_default();
        
        info!("Simulating communication from {} ({}) to {} ({})", 
            src_id, src_addr, dst_id, dst_addr);
            
        // In a real implementation, this would trigger actual communication
        // For simulation, we'll just log the attempt
        
        Ok(())
    }
    
    /// Simulate a random node failure
    async fn simulate_node_failure(&self) -> Result<(), Box<dyn Error>> {
        let mut nodes = self.nodes.lock().unwrap();
        
        if nodes.is_empty() {
            return Ok(());
        }
        
        // Select a random non-bootstrap node to fail
        let non_bootstrap: Vec<String> = nodes.iter()
            .filter(|(_, n)| n.role != "bootstrap")
            .map(|(id, _)| id.clone())
            .collect();
            
        if non_bootstrap.is_empty() {
            return Ok(());
        }
        
        let fail_idx = rand::random::<usize>() % non_bootstrap.len();
        let fail_id = &non_bootstrap[fail_idx];
        
        info!("Simulating failure of node: {}", fail_id);
        
        // Remove the node
        if let Some(node) = nodes.remove(fail_id) {
            // In a real implementation, we would actually terminate the node
            // For simulation, we just remove it from our tracking
            info!("Node {} has failed", fail_id);
            
            // After a delay, restore the node
            let node_id = fail_id.clone();
            let node_config = self.config.nodes.get(&node_id).unwrap().clone();
            let testnet = self.clone();
            
            tokio::spawn(async move {
                sleep(Duration::from_secs(10)).await;
                match testnet.start_node(&node_id, &node_config).await {
                    Ok(_) => info!("Node {} has recovered after failure", node_id),
                    Err(e) => error!("Failed to recover node {}: {}", node_id, e),
                }
            });
        }
        
        Ok(())
    }
    
    /// Clone the testnet manager (for async tasks)
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            nodes: Arc::clone(&self.nodes),
            node_addresses: Arc::clone(&self.node_addresses),
        }
    }
}

// Add cloning for TestnetConfig
impl Clone for TestnetConfig {
    fn clone(&self) -> Self {
        Self {
            testnet: TestnetInfo {
                name: self.testnet.name.clone(),
                description: self.testnet.description.clone(),
                version: self.testnet.version.clone(),
            },
            network: NetworkConfig {
                bootstrap_nodes: self.network.bootstrap_nodes.clone(),
                min_peers: self.network.min_peers,
                max_peers: self.network.max_peers,
                connection_timeout_ms: self.network.connection_timeout_ms,
                peer_discovery_interval_ms: self.network.peer_discovery_interval_ms,
                overlay: OverlayConfig {
                    address_space: self.network.overlay.address_space.clone(),
                    allocation_strategy: self.network.overlay.allocation_strategy.clone(),
                    federation_prefix_len: self.network.overlay.federation_prefix_len,
                    node_prefix_len: self.network.overlay.node_prefix_len,
                    default_tunnel_type: self.network.overlay.default_tunnel_type.clone(),
                },
            },
            federations: self.federations.clone(),
            nodes: self.nodes.clone(),
            simulation: self.simulation.clone(),
        }
    }
}

// Add cloning for other types
impl Clone for FederationConfig {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            description: self.description.clone(),
            bootstrap_nodes: self.bootstrap_nodes.clone(),
            min_nodes: self.min_nodes,
            forwarding_policy: self.forwarding_policy.clone(),
        }
    }
}

impl Clone for NodeConfig {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            federation: self.federation.clone(),
            role: self.role.clone(),
            listen_port: self.listen_port,
            forwarding_policy: self.forwarding_policy.clone(),
            log_level: self.log_level.clone(),
        }
    }
}

impl Clone for SimulationConfig {
    fn clone(&self) -> Self {
        Self {
            enabled: self.enabled,
            duration_seconds: self.duration_seconds,
            message_interval_ms: self.message_interval_ms,
            failure_probability: self.failure_probability,
            latency_min_ms: self.latency_min_ms,
            latency_max_ms: self.latency_max_ms,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command line arguments
    let matches = App::new("ICN IPv6 Overlay Network Testnet")
        .version("0.1.0")
        .author("ICN Network Team")
        .about("Launches a testnet for the ICN IPv6 overlay network")
        .arg(Arg::with_name("config")
            .short("c")
            .long("config")
            .value_name("FILE")
            .help("Path to testnet configuration file")
            .takes_value(true)
            .default_value("testnet/config/testnet.toml"))
        .arg(Arg::with_name("no-simulation")
            .long("no-simulation")
            .help("Disable simulation"))
        .get_matches();
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");
    
    // Get config path
    let config_path = matches.value_of("config").unwrap();
    info!("Starting testnet with config: {}", config_path);
    
    // Create testnet manager
    let testnet = TestnetManager::new(config_path)?;
    
    // Start testnet
    testnet.start_testnet().await?;
    
    // Run simulation if enabled and not disabled via flag
    if !matches.is_present("no-simulation") {
        testnet.run_simulation().await?;
    }
    
    // Keep testnet running until Ctrl+C
    info!("Testnet is running. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;
    info!("Shutting down testnet...");
    
    Ok(())
} 