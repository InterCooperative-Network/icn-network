//! Peer discovery mechanisms for the ICN network
//!
//! This module provides various peer discovery mechanisms to find
//! and connect to other nodes in the InterCooperative Network.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};

use icn_core::storage::Storage;
use crate::{NetworkResult, NetworkError, NetworkService, PeerInfo};

/// The default storage key for saved peers
const SAVED_PEERS_KEY: &str = "network/saved_peers";

/// A peer discovery mechanism
#[async_trait]
pub trait PeerDiscovery: Send + Sync {
    /// Start the discovery mechanism
    async fn start(&self) -> NetworkResult<()>;
    
    /// Stop the discovery mechanism
    async fn stop(&self) -> NetworkResult<()>;
    
    /// Get a list of discovered peers
    async fn get_discovered_peers(&self) -> NetworkResult<Vec<(PeerId, Multiaddr)>>;
}

/// Discovery config
#[derive(Clone, Debug)]
pub struct DiscoveryConfig {
    /// Whether to use mDNS discovery
    pub use_mdns: bool,
    /// Whether to use Kademlia discovery
    pub use_kademlia: bool,
    /// Whether to use bootstrap servers
    pub use_bootstrap: bool,
    /// List of bootstrap servers
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Whether to save discovered peers
    pub save_peers: bool,
    /// How often to retry connecting to peers (in seconds)
    pub retry_interval: u64,
    /// Maximum number of peers to remember
    pub max_saved_peers: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            use_mdns: true,
            use_kademlia: true,
            use_bootstrap: true,
            bootstrap_peers: Vec::new(),
            save_peers: true,
            retry_interval: 60,
            max_saved_peers: 100,
        }
    }
}

/// The main discovery manager
pub struct DiscoveryManager {
    /// Network service
    network: Arc<dyn NetworkService>,
    /// Storage for saving discovered peers
    storage: Arc<dyn Storage>,
    /// Configuration
    config: DiscoveryConfig,
    /// Known peers
    known_peers: Arc<RwLock<HashSet<(PeerId, Multiaddr)>>>,
    /// Running flag
    running: Arc<RwLock<bool>>,
}

impl DiscoveryManager {
    /// Create a new discovery manager
    pub fn new(
        network: Arc<dyn NetworkService>,
        storage: Arc<dyn Storage>,
        config: DiscoveryConfig,
    ) -> Self {
        Self {
            network,
            storage,
            config,
            known_peers: Arc::new(RwLock::new(HashSet::new())),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Load saved peers from storage
    async fn load_saved_peers(&self) -> NetworkResult<()> {
        if !self.config.save_peers {
            return Ok(());
        }
        
        if !self.storage.exists(SAVED_PEERS_KEY).await? {
            return Ok(());
        }
        
        // Load the saved peers
        let data = self.storage.get(SAVED_PEERS_KEY).await?;
        let peers: Vec<(String, String)> = match serde_json::from_slice(&data) {
            Ok(peers) => peers,
            Err(e) => {
                warn!("Failed to deserialize saved peers: {}", e);
                return Ok(());
            }
        };
        
        let mut known_peers = self.known_peers.write().await;
        
        // Process each peer
        for (peer_id_str, addr_str) in peers {
            // Parse the peer ID
            let peer_id = match PeerId::from_bytes(&hex::decode(&peer_id_str).unwrap_or_default()) {
                Ok(peer_id) => peer_id,
                Err(e) => {
                    warn!("Failed to parse peer ID {}: {}", peer_id_str, e);
                    continue;
                }
            };
            
            // Parse the multiaddr
            let addr = match addr_str.parse::<Multiaddr>() {
                Ok(addr) => addr,
                Err(e) => {
                    warn!("Failed to parse multiaddr {}: {}", addr_str, e);
                    continue;
                }
            };
            
            // Add to known peers
            known_peers.insert((peer_id, addr));
        }
        
        info!("Loaded {} saved peers", known_peers.len());
        
        Ok(())
    }
    
    /// Save known peers to storage
    async fn save_peers(&self) -> NetworkResult<()> {
        if !self.config.save_peers {
            return Ok(());
        }
        
        let known_peers = self.known_peers.read().await;
        
        // Convert to a serializable format
        let peers: Vec<(String, String)> = known_peers
            .iter()
            .map(|(peer_id, addr)| {
                (
                    hex::encode(peer_id.to_bytes()),
                    addr.to_string(),
                )
            })
            .collect();
        
        // Serialize and save
        let data = serde_json::to_vec(&peers)
            .map_err(|e| NetworkError::SerializationError(e.to_string()))?;
        
        self.storage.put(SAVED_PEERS_KEY, &data).await?;
        
        info!("Saved {} peers", peers.len());
        
        Ok(())
    }
    
    /// Add a new discovered peer
    async fn add_discovered_peer(&self, peer_id: PeerId, addr: Multiaddr) -> NetworkResult<bool> {
        let mut known_peers = self.known_peers.write().await;
        
        // Check if we already know this peer
        let entry = (peer_id, addr.clone());
        if known_peers.contains(&entry) {
            return Ok(false);
        }
        
        // Add to known peers
        known_peers.insert(entry);
        
        // Limit the size of the known peers set
        if known_peers.len() > self.config.max_saved_peers {
            // Remove a random peer (in a real implementation, this would be more sophisticated)
            if let Some(peer) = known_peers.iter().next().cloned() {
                known_peers.remove(&peer);
            }
        }
        
        // Save the updated peers
        drop(known_peers);
        self.save_peers().await?;
        
        Ok(true)
    }
    
    /// Connect to a peer
    async fn connect_to_peer(&self, peer_id: PeerId, addr: Multiaddr) -> NetworkResult<()> {
        // Check if we're already connected
        let peers = self.network.get_connected_peers().await?;
        if peers.iter().any(|p| p.peer_id == peer_id) {
            return Ok(());
        }
        
        // Try to connect
        match self.network.connect(&addr).await {
            Ok(_) => {
                info!("Connected to peer {} at {}", peer_id, addr);
                Ok(())
            }
            Err(e) => {
                warn!("Failed to connect to peer {} at {}: {}", peer_id, addr, e);
                Err(e)
            }
        }
    }
    
    /// Periodic task to try connecting to known peers
    async fn run_connection_task(&self) {
        let running = self.running.clone();
        let network = self.network.clone();
        let known_peers = self.known_peers.clone();
        let retry_interval = self.config.retry_interval;
        
        tokio::spawn(async move {
            info!("Starting connection task");
            
            while *running.read().await {
                // Get list of known peers
                let peers = {
                    let known = known_peers.read().await;
                    known.clone()
                };
                
                // Get currently connected peers
                let connected = match network.get_connected_peers().await {
                    Ok(peers) => peers.iter().map(|p| p.peer_id).collect::<HashSet<_>>(),
                    Err(e) => {
                        error!("Failed to get connected peers: {}", e);
                        HashSet::new()
                    }
                };
                
                // Try to connect to peers that are not already connected
                for (peer_id, addr) in peers {
                    if !connected.contains(&peer_id) {
                        debug!("Trying to connect to known peer {} at {}", peer_id, addr);
                        let _ = network.connect(&addr).await;
                    }
                }
                
                // Wait before trying again
                sleep(Duration::from_secs(retry_interval)).await;
            }
            
            info!("Connection task stopped");
        });
    }
}

#[async_trait]
impl PeerDiscovery for DiscoveryManager {
    async fn start(&self) -> NetworkResult<()> {
        // Load saved peers
        self.load_saved_peers().await?;
        
        // Connect to bootstrap peers
        if self.config.use_bootstrap {
            for addr in &self.config.bootstrap_peers {
                match self.network.connect(addr).await {
                    Ok(peer_id) => {
                        info!("Connected to bootstrap peer {} at {}", peer_id, addr);
                        self.add_discovered_peer(peer_id, addr.clone()).await?;
                    }
                    Err(e) => {
                        warn!("Failed to connect to bootstrap peer {}: {}", addr, e);
                    }
                }
            }
        }
        
        // Start the connection task
        {
            let mut running = self.running.write().await;
            *running = true;
        }
        self.run_connection_task().await;
        
        Ok(())
    }
    
    async fn stop(&self) -> NetworkResult<()> {
        // Stop the connection task
        {
            let mut running = self.running.write().await;
            *running = false;
        }
        
        // Save peers before stopping
        self.save_peers().await?;
        
        Ok(())
    }
    
    async fn get_discovered_peers(&self) -> NetworkResult<Vec<(PeerId, Multiaddr)>> {
        let known_peers = self.known_peers.read().await;
        Ok(known_peers.iter().cloned().collect())
    }
} 