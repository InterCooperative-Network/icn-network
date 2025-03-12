use crate::{
    error::{NetworkError, Result},
    node::{Message, NodeConfig, NodeType, PeerInfo},
};
use std::{
    collections::{HashMap, HashSet},
    net::SocketAddr,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};
use tokio::{
    net::UdpSocket,
    sync::mpsc,
    time,
};
use serde::{Serialize, Deserialize};
use tracing::{debug, info, warn, error};

/// Time-to-live for discovery messages
const DISCOVERY_TTL: u8 = 3;

/// Discovery protocol version
const PROTOCOL_VERSION: u8 = 1;

/// Discovery message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMessage {
    /// Request for peers
    DiscoveryRequest {
        /// Protocol version
        protocol_version: u8,
        /// Requesting node ID
        node_id: String,
        /// Cooperative ID
        coop_id: String,
        /// Federation ID (optional)
        federation_id: Option<String>,
        /// Node type
        node_type: NodeType,
        /// Time-to-live
        ttl: u8,
        /// Nonce to prevent replay
        nonce: u64,
    },
    /// Response with known peers
    DiscoveryResponse {
        /// Protocol version
        protocol_version: u8,
        /// Responding node ID
        node_id: String,
        /// Known peers
        peers: Vec<PeerInfo>,
        /// Referencing nonce from request
        ref_nonce: u64,
    },
    /// Federation announcement
    FederationAnnouncement {
        /// Protocol version
        protocol_version: u8,
        /// Federation ID
        federation_id: String,
        /// Federation description
        description: String,
        /// Bootstrap nodes
        bootstrap_nodes: Vec<SocketAddr>,
        /// Services offered
        services: Vec<String>,
    },
}

/// Discovery service for finding peers and federations
pub struct DiscoveryService {
    /// Node configuration
    config: NodeConfig,
    
    /// Known peers
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    
    /// Known federations
    federations: Arc<RwLock<HashMap<String, FederationInfo>>>,
    
    /// Bootstrap nodes
    bootstrap_nodes: Vec<SocketAddr>,
    
    /// Discovery interval
    discovery_interval: Duration,
    
    /// Channel for sending messages to the network service
    message_tx: mpsc::Sender<Message>,
    
    /// Recently seen nonces (to prevent replay)
    recent_nonces: Arc<RwLock<HashSet<u64>>>,
    
    /// Last discovery time
    last_discovery: Arc<RwLock<Instant>>,
    
    /// Is running flag
    running: Arc<RwLock<bool>>,
}

/// Information about a federation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationInfo {
    /// Federation ID
    pub federation_id: String,
    
    /// Federation description
    pub description: String,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<SocketAddr>,
    
    /// Services offered
    pub services: Vec<String>,
    
    /// Last seen timestamp
    pub last_seen: u64,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub fn new(
        config: NodeConfig,
        peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
        bootstrap_nodes: Vec<SocketAddr>,
        message_tx: mpsc::Sender<Message>,
    ) -> Self {
        let discovery_interval = config.discovery_interval.unwrap_or(Duration::from_secs(30));
        
        Self {
            config,
            peers,
            federations: Arc::new(RwLock::new(HashMap::new())),
            bootstrap_nodes,
            discovery_interval,
            message_tx,
            recent_nonces: Arc::new(RwLock::new(HashSet::new())),
            last_discovery: Arc::new(RwLock::new(Instant::now())),
            running: Arc::new(RwLock::new(false)),
        }
    }
    
    /// Start the discovery service
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().map_err(|_| NetworkError::LockError)?;
        if *running {
            return Ok(());
        }
        
        *running = true;
        
        // Clone references for the background task
        let peers = Arc::clone(&self.peers);
        let running = Arc::clone(&self.running);
        let last_discovery = Arc::clone(&self.last_discovery);
        let discovery_interval = self.discovery_interval;
        let bootstrap_nodes = self.bootstrap_nodes.clone();
        let message_tx = self.message_tx.clone();
        let config = self.config.clone();
        
        // Spawn background task for periodic discovery
        tokio::spawn(async move {
            let mut interval = time::interval(discovery_interval);
            
            loop {
                interval.tick().await;
                
                if !*running.read().unwrap() {
                    break;
                }
                
                // Update last discovery time
                *last_discovery.write().unwrap() = Instant::now();
                
                // Send discovery requests to bootstrap nodes
                for node_addr in &bootstrap_nodes {
                    if let Err(e) = Self::send_discovery_request(
                        &message_tx,
                        node_addr,
                        &config,
                    ).await {
                        warn!("Failed to send discovery request to {}: {}", node_addr, e);
                    }
                }
                
                // Also send discovery requests to some known peers
                let peers_snapshot = {
                    let peers_guard = peers.read().unwrap();
                    peers_guard.values().cloned().collect::<Vec<_>>()
                };
                
                // Choose a subset of peers to query (for larger networks)
                // Here we just use the first 5 for simplicity
                for peer in peers_snapshot.iter().take(5) {
                    if let Err(e) = Self::send_discovery_request(
                        &message_tx,
                        &peer.addr,
                        &config,
                    ).await {
                        warn!("Failed to send discovery request to {}: {}", peer.addr, e);
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the discovery service
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().map_err(|_| NetworkError::LockError)?;
        *running = false;
        Ok(())
    }
    
    /// Send a discovery request to a node
    async fn send_discovery_request(
        message_tx: &mpsc::Sender<Message>,
        target: &SocketAddr,
        config: &NodeConfig,
    ) -> Result<()> {
        let nonce = rand::random::<u64>();
        
        let discovery_request = DiscoveryMessage::DiscoveryRequest {
            protocol_version: PROTOCOL_VERSION,
            node_id: config.node_id.clone(),
            coop_id: config.coop_id.clone(),
            federation_id: None, // TODO: Add federation ID
            node_type: config.node_type.clone(),
            ttl: DISCOVERY_TTL,
            nonce,
        };
        
        // Serialize the discovery request
        let serialized = bincode::serialize(&discovery_request)
            .map_err(|e| NetworkError::Serialization(e))?;
        
        // Send the discovery request
        message_tx.send(Message::Data(serialized)).await
            .map_err(|_| NetworkError::ChannelClosed)?;
        
        Ok(())
    }
    
    /// Process a discovery response
    pub async fn process_discovery_response(
        &self,
        source: SocketAddr,
        message: &[u8],
    ) -> Result<()> {
        // Deserialize the discovery message
        let discovery_message: DiscoveryMessage = bincode::deserialize(message)
            .map_err(|e| NetworkError::Serialization(e))?;
        
        match discovery_message {
            DiscoveryMessage::DiscoveryRequest { 
                node_id, 
                coop_id, 
                node_type, 
                ttl, 
                nonce, 
                ..
            } => {
                // Process the discovery request
                self.handle_discovery_request(source, node_id, coop_id, node_type, ttl, nonce).await?;
            },
            DiscoveryMessage::DiscoveryResponse { 
                node_id, 
                peers, 
                ref_nonce, 
                .. 
            } => {
                // Process the discovery response
                self.handle_discovery_response(node_id, peers, ref_nonce).await?;
            },
            DiscoveryMessage::FederationAnnouncement { 
                federation_id, 
                description, 
                bootstrap_nodes, 
                services, 
                .. 
            } => {
                // Process the federation announcement
                self.handle_federation_announcement(federation_id, description, bootstrap_nodes, services).await?;
            }
        }
        
        Ok(())
    }
    
    /// Handle a discovery request
    async fn handle_discovery_request(
        &self,
        source: SocketAddr,
        node_id: String,
        coop_id: String,
        node_type: NodeType,
        ttl: u8,
        nonce: u64,
    ) -> Result<()> {
        // Check if we've seen this nonce recently
        {
            let mut recent_nonces = self.recent_nonces.write().map_err(|_| NetworkError::LockError)?;
            if recent_nonces.contains(&nonce) {
                // Already processed this request, ignore
                return Ok(());
            }
            
            // Add to recent nonces
            recent_nonces.insert(nonce);
        }
        
        // Add the requesting node to our peer list
        let peer_info = PeerInfo {
            node_id: node_id.clone(),
            coop_id: coop_id.clone(),
            node_type: node_type.clone(),
            addr: source,
            last_seen: chrono::Utc::now().timestamp() as u64,
        };
        
        {
            let mut peers = self.peers.write().map_err(|_| NetworkError::LockError)?;
            peers.insert(node_id.clone(), peer_info);
        }
        
        // If TTL is greater than 0, forward to other peers
        if ttl > 1 {
            // This is where you would implement request forwarding
            // For simplicity, we'll skip this for now
        }
        
        // Send a discovery response
        self.send_discovery_response(source, nonce).await?;
        
        Ok(())
    }
    
    /// Send a discovery response
    async fn send_discovery_response(
        &self,
        target: SocketAddr,
        ref_nonce: u64,
    ) -> Result<()> {
        // Get a list of known peers
        let peers = {
            let peers_guard = self.peers.read().map_err(|_| NetworkError::LockError)?;
            peers_guard.values().cloned().collect::<Vec<_>>()
        };
        
        let response = DiscoveryMessage::DiscoveryResponse {
            protocol_version: PROTOCOL_VERSION,
            node_id: self.config.node_id.clone(),
            peers,
            ref_nonce,
        };
        
        // Serialize the response
        let serialized = bincode::serialize(&response)
            .map_err(|e| NetworkError::Serialization(e))?;
        
        // Send the response
        self.message_tx.send(Message::Data(serialized)).await
            .map_err(|_| NetworkError::ChannelClosed)?;
        
        Ok(())
    }
    
    /// Handle a discovery response
    async fn handle_discovery_response(
        &self,
        node_id: String,
        peers: Vec<PeerInfo>,
        ref_nonce: u64,
    ) -> Result<()> {
        // Verify that we requested this response
        // In a real implementation, we'd check if ref_nonce matches a nonce we sent
        
        // Add the peers to our peer list
        let mut our_peers = self.peers.write().map_err(|_| NetworkError::LockError)?;
        
        for peer in peers {
            our_peers.insert(peer.node_id.clone(), peer);
        }
        
        debug!("Updated peer list from discovery response, now have {} peers", our_peers.len());
        
        Ok(())
    }
    
    /// Handle a federation announcement
    async fn handle_federation_announcement(
        &self,
        federation_id: String,
        description: String,
        bootstrap_nodes: Vec<SocketAddr>,
        services: Vec<String>,
    ) -> Result<()> {
        let federation_info = FederationInfo {
            federation_id: federation_id.clone(),
            description,
            bootstrap_nodes,
            services,
            last_seen: chrono::Utc::now().timestamp() as u64,
        };
        
        let mut federations = self.federations.write().map_err(|_| NetworkError::LockError)?;
        federations.insert(federation_id.clone(), federation_info);
        
        info!("Added or updated federation: {}", federation_id);
        
        Ok(())
    }
    
    /// Announce a federation to the network
    pub async fn announce_federation(
        &self,
        federation_id: String,
        description: String,
        bootstrap_nodes: Vec<SocketAddr>,
        services: Vec<String>,
    ) -> Result<()> {
        let announcement = DiscoveryMessage::FederationAnnouncement {
            protocol_version: PROTOCOL_VERSION,
            federation_id,
            description,
            bootstrap_nodes,
            services,
        };
        
        // Serialize the announcement
        let serialized = bincode::serialize(&announcement)
            .map_err(|e| NetworkError::Serialization(e))?;
        
        // Send to all known peers
        let peers = {
            let peers_guard = self.peers.read().map_err(|_| NetworkError::LockError)?;
            peers_guard.values().cloned().collect::<Vec<_>>()
        };
        
        for peer in peers {
            self.message_tx.send(Message::Data(serialized.clone())).await
                .map_err(|_| NetworkError::ChannelClosed)?;
        }
        
        Ok(())
    }
    
    /// Get known federations
    pub fn get_federations(&self) -> Result<Vec<FederationInfo>> {
        let federations = self.federations.read().map_err(|_| NetworkError::LockError)?;
        Ok(federations.values().cloned().collect())
    }
    
    /// Find a federation by ID
    pub fn find_federation(&self, federation_id: &str) -> Result<Option<FederationInfo>> {
        let federations = self.federations.read().map_err(|_| NetworkError::LockError)?;
        Ok(federations.get(federation_id).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;
    
    #[tokio::test]
    async fn test_discovery_service_creation() {
        let config = NodeConfig {
            listen_addr: "127.0.0.1:8000".parse().unwrap(),
            peers: vec![],
            node_id: "test-node".to_string(),
            coop_id: "test-coop".to_string(),
            node_type: NodeType::Primary,
            discovery_interval: Some(Duration::from_secs(30)),
            health_check_interval: None,
        };
        
        let peers = Arc::new(RwLock::new(HashMap::new()));
        let bootstrap_nodes = vec!["127.0.0.1:8001".parse().unwrap()];
        let (tx, _rx) = mpsc::channel(100);
        
        let discovery = DiscoveryService::new(
            config,
            peers,
            bootstrap_nodes,
            tx,
        );
        
        assert_eq!(discovery.discovery_interval, Duration::from_secs(30));
        assert_eq!(discovery.bootstrap_nodes.len(), 1);
    }
} 