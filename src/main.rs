use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// Modules for our ICN node
mod config;
mod identity;
mod networking;
mod storage;
mod crypto;
mod economic;

use config::NodeConfig;
use identity::Identity;
use networking::{NetworkManager, PeerInfo};
use storage::Storage;
use economic::MutualCreditSystem;

// Main ICN Node structure
pub struct IcnNode {
    config: NodeConfig,
    identity: Identity,
    network: NetworkManager,
    storage: Storage,
    economic: MutualCreditSystem,
    peers: Arc<Mutex<Vec<PeerInfo>>>,
}

impl IcnNode {
    // Create a new ICN node
    pub fn new(config: NodeConfig) -> Result<Self, Box<dyn Error>> {
        println!("Initializing ICN Node: {}", config.node_id);
        
        // Initialize components
        let identity = Identity::new(&config.node_id, &config.coop_id)?;
        let storage = Storage::new(&config.data_dir)?;
        let network = NetworkManager::new(config.listen_addr.parse::<SocketAddr>()?, config.tls.clone())?;
        let economic = MutualCreditSystem::new(identity.clone(), storage.clone());
        let peers = Arc::new(Mutex::new(Vec::new()));
        
        Ok(IcnNode {
            config,
            identity,
            network,
            storage,
            economic,
            peers,
        })
    }
    
    // Start the ICN node
    pub fn start(&mut self) -> Result<(), Box<dyn Error>> {
        println!("Starting ICN Node: {}", self.config.node_id);
        
        // Start the network manager
        self.network.start()?;
        
        // Connect to initial peers if provided
        if !self.config.peers.is_empty() {
            for peer_addr in &self.config.peers {
                match peer_addr.parse::<SocketAddr>() {
                    Ok(addr) => {
                        println!("Connecting to peer: {}", addr);
                        let _ = self.network.connect_to_peer(addr);
                    },
                    Err(e) => println!("Invalid peer address: {} - {}", peer_addr, e),
                }
            }
        }
        
        // Start periodic tasks
        self.start_discovery();
        self.start_health_check();
        
        println!("ICN Node started successfully");
        println!("Ready to facilitate transactions between cooperative members");
        
        // Keep the main thread alive
        loop {
            std::thread::sleep(Duration::from_secs(10));
            println!("ICN Node is running...");
        }
    }
    
    // Start peer discovery process
    fn start_discovery(&self) {
        let peers_clone = Arc::clone(&self.peers);
        let discovery_interval = self.config.discovery_interval;
        
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(discovery_interval));
                println!("Running peer discovery...");
                println!("Connected peers: {}", peers_clone.lock().unwrap().len());
            }
        });
    }
    
    // Start health check process
    fn start_health_check(&self) {
        let peers_clone = Arc::clone(&self.peers);
        let health_check_interval = self.config.health_check_interval;
        
        std::thread::spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(health_check_interval));
                println!("Running health check...");
                let healthy_peers = peers_clone.lock().unwrap().len();
                println!("Healthy peers: {}", healthy_peers);
            }
        });
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Load configuration from environment or file
    let config = NodeConfig::from_env()?;
    
    // Create and start the ICN node
    let mut node = IcnNode::new(config)?;
    node.start()?;
    
    Ok(())
} 