use std::env;
use std::error::Error;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{info, error};
use tokio::signal;
use log::{info, error};

// Modules for our ICN node
mod config;
mod identity;
mod networking;
mod storage;
mod crypto;
mod economic;
mod federation;
mod federation_governance;
mod cross_federation_governance;
mod resource_sharing;

use config::NodeConfig;
use identity::Identity;
use networking::{NetworkManager, PeerInfo};
use storage::Storage;
use crypto::CryptoUtils;
use economic::MutualCreditSystem;
use federation::FederationSystem;
use federation_governance::FederationGovernance;
use cross_federation_governance::CrossFederationGovernance;
use resource_sharing::ResourceSharingSystem;

// Main ICN Node structure
pub struct IcnNode {
    config: NodeConfig,
    identity: Arc<Identity>,
    network: NetworkManager,
    storage: Arc<Storage>,
    economic: Arc<MutualCreditSystem>,
    federation: Arc<FederationSystem>,
    governance: Arc<FederationGovernance>,
    cross_federation_governance: Arc<CrossFederationGovernance>,
    resource_sharing: Arc<ResourceSharingSystem>,
    peers: Arc<Mutex<Vec<PeerInfo>>>,
}

impl IcnNode {
    // Create a new ICN node
    pub async fn new(
        coop_id: String,
        node_id: String,
        did: String,
        storage_path: std::path::PathBuf,
    ) -> Result<Self, Box<dyn Error>> {
        info!("Initializing ICN Node...");
        
        // Initialize components
        let storage = Arc::new(Storage::new(storage_path)?);
        let identity = Arc::new(Identity::new(
            coop_id.clone(),
            node_id.clone(),
            did.clone(),
            storage.clone(),
        )?);
        let network = NetworkManager::new(identity.listen_addr.parse::<SocketAddr>()?, identity.tls.clone())?;
        let crypto = Arc::new(CryptoUtils::new());
        let economic = Arc::new(MutualCreditSystem::new(
            identity.clone(),
            storage.clone(),
            crypto.clone(),
        ));
        let federation = Arc::new(FederationSystem::new(
            identity.clone(),
            storage.clone(),
            economic.clone(),
        ));
        let governance = Arc::new(FederationGovernance::new(
            identity.clone(),
            storage.clone(),
        ));
        let cross_federation_governance = Arc::new(CrossFederationGovernance::new(
            identity.clone(),
            storage.clone(),
        ));
        let resource_sharing = Arc::new(ResourceSharingSystem::new(
            identity.clone(),
            storage.clone(),
        ));
        let peers = Arc::new(Mutex::new(Vec::new()));
        
        Ok(IcnNode {
            config: NodeConfig::new(coop_id, node_id, did, identity.listen_addr.clone(), identity.tls.clone()),
            identity,
            network,
            storage,
            economic,
            federation,
            governance,
            cross_federation_governance,
            resource_sharing,
            peers,
        })
    }
    
    // Start the ICN node
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        info!("Starting ICN Node: {}", self.identity.node_id);
        info!("Cooperative: {}", self.identity.coop_id);
        info!("DID: {}", self.identity.did);
        
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
        
        // Initialize systems
        self.economic.start().await?;
        
        info!("Ready to facilitate transactions between cooperative members");
        info!("Ready to handle federation transactions and governance");
        info!("Ready to participate in cross-federation coordination");
        info!("Ready to manage resource sharing between federations");
        
        Ok(())
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

    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        info!("Stopping ICN Node...");
        self.economic.stop().await?;
        info!("ICN Node stopped");
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize logging
    env_logger::init();

    // Create storage directory
    let storage_path = std::path::PathBuf::from("data");
    std::fs::create_dir_all(&storage_path)?;

    // Create node
    let node = IcnNode::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "test-did:test:test-coop:test-node".to_string(),
        storage_path,
    ).await?;

    // Start node
    node.start().await?;

    // Wait for shutdown signal
    match signal::ctrl_c().await {
        Ok(()) => {
            info!("Received shutdown signal");
            node.stop().await?;
        }
        Err(err) => {
            error!("Error waiting for shutdown signal: {}", err);
        }
    }

    Ok(())
} 