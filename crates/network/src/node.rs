use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use log::{info, debug, warn, error};
use serde::{Serialize, Deserialize};

use crate::error::{Result, NetworkError};
use crate::overlay::{
    OverlayNetworkManager, OverlayNetworkService, OverlayAddress, 
    OverlayOptions, MessagePriority
};

/// Node identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(String);

impl NodeId {
    pub fn new(id: String) -> Self {
        NodeId(id)
    }
    
    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<String> for NodeId {
    fn from(s: String) -> Self {
        NodeId(s)
    }
}

impl From<&str> for NodeId {
    fn from(s: &str) -> Self {
        NodeId(s.to_string())
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Node status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is initializing
    Initializing,
    /// Node is online and functioning
    Online,
    /// Node is offline or unreachable
    Offline,
    /// Node is in maintenance mode
    Maintenance,
    /// Node has encountered an error
    Error,
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Bootstrap nodes to connect to
    pub bootstrap_nodes: Vec<String>,
    /// Federation ID this node belongs to
    pub federation_id: Option<String>,
    /// Network interface to bind to
    pub interface: Option<String>,
    /// Port to listen on
    pub port: Option<u16>,
    /// Additional configurations
    pub options: HashMap<String, String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            bootstrap_nodes: Vec::new(),
            federation_id: None,
            interface: None,
            port: None,
            options: HashMap::new(),
        }
    }
}

/// Node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier
    pub id: NodeId,
    /// Current status
    pub status: NodeStatus,
    /// IP address if known
    pub ip_address: Option<String>,
    /// Port number
    pub port: Option<u16>,
    /// Federation ID if part of a federation
    pub federation_id: Option<String>,
    /// Current overlay address if connected
    pub overlay_address: Option<OverlayAddress>,
    /// Last seen timestamp
    pub last_seen: Option<u64>,
    /// Additional attributes
    pub attributes: HashMap<String, String>,
}

/// Networking node
pub struct Node {
    /// Node identifier
    id: NodeId,
    /// Node configuration
    config: NodeConfig,
    /// Node status
    status: Arc<RwLock<NodeStatus>>,
    /// Overlay network manager
    overlay: OverlayNetworkManager,
    /// Overlay network address
    overlay_address: Option<OverlayAddress>,
    /// Connected peers
    peers: Arc<RwLock<HashMap<NodeId, NodeInfo>>>,
}

impl Node {
    pub fn new(id: NodeId, config: NodeConfig) -> Self {
        Self {
            id,
            config,
            status: Arc::new(RwLock::new(NodeStatus::Initializing)),
            overlay: OverlayNetworkManager::new(),
            overlay_address: None,
            peers: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Get the node ID
    pub fn id(&self) -> &NodeId {
        &self.id
    }
    
    /// Get the node's current status
    pub async fn status(&self) -> NodeStatus {
        self.status.read().await.clone()
    }
    
    /// Set the node's status
    pub async fn set_status(&self, status: NodeStatus) {
        let mut status_guard = self.status.write().await;
        *status_guard = status;
    }
    
    /// Initialize the overlay network
    pub async fn initialize_overlay(&mut self, federation_id: Option<String>) -> Result<OverlayAddress> {
        let federation_id_ref = federation_id.as_deref();
        let address = self.overlay.initialize(&self.id.to_string(), federation_id_ref).await?;
        
        // Store the address
        self.overlay_address = Some(address.clone());
        
        info!("Node {} initialized overlay network with address: {:?}", self.id, address);
        Ok(address)
    }
    
    /// Connect to the overlay network using bootstrap nodes
    pub async fn connect_to_overlay(&mut self, bootstrap_addresses: Vec<OverlayAddress>) -> Result<()> {
        info!("Connecting to overlay network with {} bootstrap nodes", bootstrap_addresses.len());
        self.overlay.connect(&bootstrap_addresses).await?;
        info!("Node {} connected to overlay network", self.id);
        
        Ok(())
    }
    
    /// Send data through the overlay network
    pub async fn send_overlay_message(&self, destination: &OverlayAddress, data: Vec<u8>, 
                                      anonymity_required: bool) -> Result<()> {
        let options = OverlayOptions {
            anonymity_required,
            reliability_required: true,
            priority: MessagePriority::Normal,
        };
        
        self.overlay.send_data(destination, &data, &options).await?;
        debug!("Node {} sent message to {:?} through overlay", self.id, destination);
        
        Ok(())
    }
    
    /// Get the node's overlay address
    pub fn get_overlay_address(&self) -> Option<OverlayAddress> {
        self.overlay_address.clone()
    }
    
    /// Get information about this node
    pub async fn get_node_info(&self) -> NodeInfo {
        NodeInfo {
            id: self.id.clone(),
            status: self.status.read().await.clone(),
            ip_address: None, // Would be populated in a real implementation
            port: self.config.port,
            federation_id: self.config.federation_id.clone(),
            overlay_address: self.overlay_address.clone(),
            last_seen: None,
            attributes: HashMap::new(),
        }
    }
    
    /// Register a peer
    pub async fn register_peer(&self, peer_info: NodeInfo) -> Result<()> {
        let mut peers = self.peers.write().await;
        peers.insert(peer_info.id.clone(), peer_info);
        Ok(())
    }
    
    /// Get peer information
    pub async fn get_peer(&self, peer_id: &NodeId) -> Option<NodeInfo> {
        let peers = self.peers.read().await;
        peers.get(peer_id).cloned()
    }
    
    /// Get all peers
    pub async fn get_all_peers(&self) -> Vec<NodeInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        // Clean up resources when node is dropped
        // This would be implemented for real cleanup in production
    }
} 