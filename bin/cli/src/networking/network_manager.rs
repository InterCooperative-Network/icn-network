use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use std::collections::HashMap;
use anyhow::Result;
use libp2p::{Multiaddr, PeerId};
use tokio::sync::RwLock;

use crate::storage::StorageService;
use super::wireguard::WireGuardOverlay;

// Import the network crate components
use icn_network::{
    NetworkService, P2pNetwork, P2pConfig,
    PeerInfo, NetworkMessage, NetworkError
};

/// Statistics from a network connection test
pub struct ConnectionStats {
    /// Round-trip time in milliseconds
    pub rtt_ms: u64,
    /// Connection quality score (0-10)
    pub quality: u8,
    /// Protocol version detected
    pub protocol_version: String,
    /// Discovered peers
    pub peers: Vec<DiscoveredPeer>,
}

/// Information about a discovered peer
pub struct DiscoveredPeer {
    /// Peer identifier
    pub id: String,
    /// Peer network address
    pub address: String,
}

/// Federation network configuration
#[derive(Clone, Debug)]
pub struct FederationNetworkConfig {
    /// Federation identifier
    pub federation_id: String,
    /// Bootstrap peers for this federation
    pub bootstrap_peers: Vec<String>,
    /// Whether this federation allows connections from other federations
    pub allow_cross_federation: bool,
    /// Allowed federations for cross-federation communication
    pub allowed_federations: Vec<String>,
    /// Whether to encrypt federation traffic 
    pub encrypt_traffic: bool,
    /// Whether to use WireGuard for this federation
    pub use_wireguard: bool,
    /// DHT namespace for this federation
    pub dht_namespace: String,
    /// Topic prefix for federation-specific messaging
    pub topic_prefix: String,
}

impl Default for FederationNetworkConfig {
    fn default() -> Self {
        Self {
            federation_id: "default".to_string(),
            bootstrap_peers: Vec::new(),
            allow_cross_federation: false,
            allowed_federations: Vec::new(),
            encrypt_traffic: true,
            use_wireguard: false,
            dht_namespace: "icn-default".to_string(),
            topic_prefix: "icn.default".to_string(),
        }
    }
}

/// Federation network state
struct FederationState {
    /// Federation network configuration
    config: FederationNetworkConfig,
    /// Connected peers within this federation
    peers: HashMap<PeerId, PeerInfo>,
    /// Active WireGuard overlay for this federation
    wireguard: Option<Arc<RwLock<WireGuardOverlay>>>,
    /// Federation-specific metrics
    metrics: FederationMetrics,
}

/// Federation network metrics
struct FederationMetrics {
    /// Total messages sent within federation
    messages_sent: u64,
    /// Total messages received within federation
    messages_received: u64,
    /// Cross-federation messages sent
    cross_federation_sent: u64,
    /// Cross-federation messages received
    cross_federation_received: u64,
    /// Total connected peers
    peer_count: usize,
    /// Last federation sync time
    last_sync: Option<std::time::Instant>,
}

impl FederationMetrics {
    fn new() -> Self {
        Self {
            messages_sent: 0,
            messages_received: 0,
            cross_federation_sent: 0,
            cross_federation_received: 0,
            peer_count: 0,
            last_sync: None,
        }
    }
}

/// Manager for network operations
pub struct NetworkManager {
    /// Storage service for persisting network data
    pub storage: StorageService,
    /// Underlying P2P network service
    pub network: Arc<P2pNetwork>,
    /// Active connections
    connections: Arc<RwLock<Vec<PeerId>>>,
    /// WireGuard overlay for secure tunneling
    wireguard: Option<Arc<RwLock<WireGuardOverlay>>>,
    /// Federations managed by this node
    federations: Arc<RwLock<HashMap<String, FederationState>>>,
    /// Current active federation
    active_federation: Arc<RwLock<String>>,
}

impl NetworkManager {
    /// Create a new network manager
    pub async fn new(storage: StorageService) -> Result<Self> {
        // Create default network configuration
        let config = P2pConfig {
            listen_addresses: vec!["/ip4/0.0.0.0/tcp/0".parse().unwrap()],
            enable_mdns: true,
            enable_kademlia: true,
            enable_circuit_relay: true,
            ..Default::default()
        };
        
        // Initialize the P2P network with storage
        let network = P2pNetwork::new(storage.clone().into(), config).await
            .map_err(|e| anyhow::anyhow!("Failed to initialize network: {}", e))?;
            
        // Start the network service
        network.start().await
            .map_err(|e| anyhow::anyhow!("Failed to start network: {}", e))?;
        
        // Create initial federations map with default federation
        let mut federations = HashMap::new();
        federations.insert(
            "default".to_string(),
            FederationState {
                config: FederationNetworkConfig::default(),
                peers: HashMap::new(),
                wireguard: None,
                metrics: FederationMetrics::new(),
            }
        );
        
        Ok(Self {
            storage,
            network: Arc::new(network),
            connections: Arc::new(RwLock::new(Vec::new())),
            wireguard: None,
            federations: Arc::new(RwLock::new(federations)),
            active_federation: Arc::new(RwLock::new("default".to_string())),
        })
    }
    
    /// Test connectivity to a server
    pub async fn test_connectivity(&self, server_addr: &SocketAddr) -> Result<ConnectionStats> {
        // Convert the socket address to a multiaddress
        let addr = format!("/ip4/{}/tcp/{}", server_addr.ip(), server_addr.port())
            .parse::<Multiaddr>()
            .map_err(|e| anyhow::anyhow!("Invalid address format: {}", e))?;
        
        // Try to connect to the server
        let start = tokio::time::Instant::now();
        let peer_id = self.network.connect(&addr).await
            .map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;
            
        // Calculate round-trip time
        let rtt = start.elapsed();
        let rtt_ms = rtt.as_millis() as u64;
        
        // Store the connection
        self.connections.write().await.push(peer_id);
        
        // Get information about the peer
        let peer_info = self.network.get_peer_info(&peer_id).await
            .map_err(|e| anyhow::anyhow!("Failed to get peer info: {}", e))?;
            
        // Get list of peers from the connected server
        let connected_peers = self.network.get_connected_peers().await
            .map_err(|e| anyhow::anyhow!("Failed to get connected peers: {}", e))?;
            
        // Parse discovered peers
        let peers = connected_peers.into_iter()
            .map(|p| DiscoveredPeer {
                id: p.peer_id.clone(),
                address: p.addresses.first().cloned().unwrap_or_default(),
            })
            .collect();
            
        // Calculate connection quality (based on RTT, protocol support, etc.)
        let quality = if rtt_ms < 50 { 10 }
            else if rtt_ms < 100 { 9 }
            else if rtt_ms < 200 { 8 }
            else if rtt_ms < 500 { 6 }
            else if rtt_ms < 1000 { 4 }
            else { 2 };
            
        // Get protocol version
        let protocol_version = peer_info.protocol_version.unwrap_or_else(|| "unknown".to_string());
        
        Ok(ConnectionStats {
            rtt_ms,
            quality,
            protocol_version,
            peers,
        })
    }
    
    /// Connect to a peer using its address
    pub async fn connect(&self, address: &str) -> Result<String> {
        let addr = address.parse::<Multiaddr>()
            .map_err(|e| anyhow::anyhow!("Invalid address format: {}", e))?;
            
        let peer_id = self.network.connect(&addr).await
            .map_err(|e| anyhow::anyhow!("Connection failed: {}", e))?;
            
        // Store the connection
        self.connections.write().await.push(peer_id);
        
        // Update federation peer list based on active federation
        let federation = {
            let active_fed = self.active_federation.read().await;
            active_fed.clone()
        };
        
        self.add_peer_to_federation(&federation, peer_id, addr).await?;
        
        Ok(peer_id.to_string())
    }
    
    /// Disconnect from a peer
    pub async fn disconnect(&self, peer_id_str: &str) -> Result<()> {
        let peer_id = peer_id_str.parse::<PeerId>()
            .map_err(|e| anyhow::anyhow!("Invalid peer ID: {}", e))?;
            
        self.network.disconnect(&peer_id).await
            .map_err(|e| anyhow::anyhow!("Disconnection failed: {}", e))?;
            
        // Remove from connections list
        let mut connections = self.connections.write().await;
        connections.retain(|p| p != &peer_id);
        
        // Remove from all federation peer lists
        let mut federations = self.federations.write().await;
        for (_, fed_state) in federations.iter_mut() {
            fed_state.peers.remove(&peer_id);
            fed_state.metrics.peer_count = fed_state.peers.len();
        }
        
        Ok(())
    }
    
    /// List active connections
    pub async fn list_connections(&self) -> Result<Vec<PeerInfo>> {
        self.network.get_connected_peers().await
            .map_err(|e| anyhow::anyhow!("Failed to get connections: {}", e))
    }
    
    /// Send a message to a peer
    pub async fn send_message(&self, peer_id_str: &str, message_type: &str, data: serde_json::Value) -> Result<()> {
        let peer_id = peer_id_str.parse::<PeerId>()
            .map_err(|e| anyhow::anyhow!("Invalid peer ID: {}", e))?;
            
        // Create a custom message
        let custom_message = NetworkMessage::Custom(icn_network::CustomMessage {
            message_type: message_type.to_string(),
            data: data.as_object().unwrap_or(&serde_json::Map::new()).clone(),
        });
        
        // Send the message
        self.network.send_to(&peer_id, custom_message).await
            .map_err(|e| anyhow::anyhow!("Failed to send message: {}", e))?;
            
        // Update federation metrics
        let active_fed = self.active_federation.read().await.clone();
        let mut federations = self.federations.write().await;
        
        if let Some(fed_state) = federations.get_mut(&active_fed) {
            fed_state.metrics.messages_sent += 1;
            
            // Check if this is cross-federation communication
            let is_cross_federation = !fed_state.peers.contains_key(&peer_id);
            if is_cross_federation {
                fed_state.metrics.cross_federation_sent += 1;
            }
        }
        
        Ok(())
    }
    
    /// Enable circuit relay for NAT traversal
    pub async fn enable_relay(&self) -> Result<()> {
        // Configure relay with sensible defaults
        let relay_config = icn_network::circuit_relay::CircuitRelayConfig {
            max_connections: 20,
            max_circuits: 10,
            ..Default::default()
        };
        
        self.network.enable_circuit_relay(relay_config).await
            .map_err(|e| anyhow::anyhow!("Failed to enable relay: {}", e))?;
            
        Ok(())
    }
    
    /// Connect to a peer through a relay
    pub async fn connect_via_relay(&self, relay_addr: &str, target_peer_id: &str) -> Result<String> {
        let relay_multi_addr = relay_addr.parse::<Multiaddr>()
            .map_err(|e| anyhow::anyhow!("Invalid relay address: {}", e))?;
            
        let target_peer_id = target_peer_id.parse::<PeerId>()
            .map_err(|e| anyhow::anyhow!("Invalid target peer ID: {}", e))?;
            
        let peer_id = self.network.connect_via_relay(&relay_multi_addr, &target_peer_id).await
            .map_err(|e| anyhow::anyhow!("Relay connection failed: {}", e))?;
            
        // Store the connection
        self.connections.write().await.push(peer_id);
        
        // Update federation peer list based on active federation
        let federation = {
            let active_fed = self.active_federation.read().await;
            active_fed.clone()
        };
        
        // Use the relay address as a placeholder for now
        self.add_peer_to_federation(&federation, peer_id, relay_multi_addr).await?;
        
        Ok(peer_id.to_string())
    }
    
    /// Create a WireGuard tunnel
    pub async fn create_wireguard_tunnel(&self, peer_id_str: &str) -> Result<String> {
        let peer_id = peer_id_str.parse::<PeerId>()
            .map_err(|e| anyhow::anyhow!("Invalid peer ID: {}", e))?;
        
        // Check if we have a connection to this peer
        let connected_peers = self.network.get_connected_peers().await
            .map_err(|e| anyhow::anyhow!("Failed to get connected peers: {}", e))?;
            
        if !connected_peers.iter().any(|p| p.peer_id == peer_id.to_string()) {
            return Err(anyhow::anyhow!("Not connected to peer {}. Connect first before creating a tunnel.", peer_id));
        }
        
        // Initialize WireGuard if not already initialized
        let wireguard = match &self.wireguard {
            Some(wg) => wg.clone(),
            None => {
                // Create a new WireGuard interface with a random name
                let interface_name = format!("wg-icn-{}", rand::random::<u16>());
                let listen_port = 51820 + rand::random::<u16>() % 100;
                
                let wg = WireGuardOverlay::new(&interface_name, listen_port).await?;
                let wg_arc = Arc::new(RwLock::new(wg));
                
                // Temporarily set the wireguard field
                let wg_clone = wg_arc.clone();
                self.wireguard = Some(wg_arc);
                
                wg_clone
            }
        };
        
        // Get the peer's address
        let peer_addr = connected_peers.iter()
            .find(|p| p.peer_id == peer_id.to_string())
            .and_then(|p| p.addresses.first())
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Cannot find address for peer {}", peer_id))?;
            
        // Create the tunnel
        let mut wg = wireguard.write().await;
        let tunnel_name = wg.add_peer(peer_id, peer_addr.parse()?, vec![]).await?;
        
        Ok(tunnel_name)
    }
    
    /// Create a new federation
    pub async fn create_federation(&self, federation_id: &str, config: FederationNetworkConfig) -> Result<()> {
        let mut federations = self.federations.write().await;
        
        // Check if federation already exists
        if federations.contains_key(federation_id) {
            return Err(anyhow::anyhow!("Federation {} already exists", federation_id));
        }
        
        // Create new federation state
        let fed_state = FederationState {
            config,
            peers: HashMap::new(),
            wireguard: None,
            metrics: FederationMetrics::new(),
        };
        
        // Add federation to map
        federations.insert(federation_id.to_string(), fed_state);
        
        Ok(())
    }
    
    /// Get federations managed by this node
    pub async fn get_federations(&self) -> Vec<String> {
        let federations = self.federations.read().await;
        federations.keys().cloned().collect()
    }
    
    /// Switch active federation
    pub async fn set_active_federation(&self, federation_id: &str) -> Result<()> {
        let federations = self.federations.read().await;
        
        // Check if federation exists
        if !federations.contains_key(federation_id) {
            return Err(anyhow::anyhow!("Federation {} does not exist", federation_id));
        }
        
        // Set active federation
        let mut active_fed = self.active_federation.write().await;
        *active_fed = federation_id.to_string();
        
        Ok(())
    }
    
    /// Get current active federation
    pub async fn get_active_federation(&self) -> String {
        self.active_federation.read().await.clone()
    }
    
    /// Get federation configuration
    pub async fn get_federation_config(&self, federation_id: &str) -> Result<FederationNetworkConfig> {
        let federations = self.federations.read().await;
        
        // Check if federation exists
        match federations.get(federation_id) {
            Some(fed_state) => Ok(fed_state.config.clone()),
            None => Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        }
    }
    
    /// Update federation configuration
    pub async fn update_federation_config(&self, federation_id: &str, config: FederationNetworkConfig) -> Result<()> {
        let mut federations = self.federations.write().await;
        
        // Check if federation exists
        match federations.get_mut(federation_id) {
            Some(fed_state) => {
                fed_state.config = config;
                Ok(())
            },
            None => Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        }
    }
    
    /// Get federation metrics
    pub async fn get_federation_metrics(&self, federation_id: &str) -> Result<serde_json::Value> {
        let federations = self.federations.read().await;
        
        // Check if federation exists
        match federations.get(federation_id) {
            Some(fed_state) => {
                let metrics = &fed_state.metrics;
                
                // Convert metrics to JSON
                let metrics_json = serde_json::json!({
                    "messages_sent": metrics.messages_sent,
                    "messages_received": metrics.messages_received,
                    "cross_federation_sent": metrics.cross_federation_sent,
                    "cross_federation_received": metrics.cross_federation_received,
                    "peer_count": metrics.peer_count,
                    "last_sync": metrics.last_sync.map(|t| t.elapsed().as_secs()).unwrap_or(0),
                });
                
                Ok(metrics_json)
            },
            None => Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        }
    }
    
    /// Get federation peers
    pub async fn get_federation_peers(&self, federation_id: &str) -> Result<Vec<PeerInfo>> {
        let federations = self.federations.read().await;
        
        // Check if federation exists
        match federations.get(federation_id) {
            Some(fed_state) => {
                let peers = fed_state.peers.values().cloned().collect();
                Ok(peers)
            },
            None => Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        }
    }
    
    /// Enable WireGuard for a specific federation
    pub async fn enable_federation_wireguard(&self, federation_id: &str) -> Result<()> {
        let mut federations = self.federations.write().await;
        
        // Check if federation exists
        let fed_state = match federations.get_mut(federation_id) {
            Some(fed_state) => fed_state,
            None => return Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        };
        
        // Create a new WireGuard interface with federation-specific name
        let interface_name = format!("wg-icn-{}", federation_id);
        let listen_port = 51820 + rand::random::<u16>() % 100;
        
        let wg = WireGuardOverlay::new(&interface_name, listen_port).await?;
        let wg_arc = Arc::new(RwLock::new(wg));
        
        // Set federation wireguard
        fed_state.wireguard = Some(wg_arc);
        
        // Update federation config
        fed_state.config.use_wireguard = true;
        
        Ok(())
    }
    
    /// Send message to all peers in a federation
    pub async fn broadcast_to_federation(&self, federation_id: &str, message_type: &str, data: serde_json::Value) -> Result<()> {
        let federations = self.federations.read().await;
        
        // Check if federation exists
        let fed_state = match federations.get(federation_id) {
            Some(fed_state) => fed_state,
            None => return Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        };
        
        // Create a custom message
        let custom_message = NetworkMessage::Custom(icn_network::CustomMessage {
            message_type: message_type.to_string(),
            data: data.as_object().unwrap_or(&serde_json::Map::new()).clone(),
        });
        
        // Get all peer IDs in this federation
        let peer_ids: Vec<PeerId> = fed_state.peers.keys().cloned().collect();
        
        // Drop the read lock to avoid deadlock
        drop(federations);
        
        // Send message to all peers
        for peer_id in peer_ids {
            if let Err(e) = self.network.send_to(&peer_id, custom_message.clone()).await {
                println!("Failed to send message to peer {}: {}", peer_id, e);
            }
        }
        
        // Update federation metrics
        let mut federations = self.federations.write().await;
        if let Some(fed_state) = federations.get_mut(federation_id) {
            fed_state.metrics.messages_sent += 1;
        }
        
        Ok(())
    }
    
    /// Helper function to add a peer to a federation
    async fn add_peer_to_federation(&self, federation_id: &str, peer_id: PeerId, addr: Multiaddr) -> Result<()> {
        let mut federations = self.federations.write().await;
        
        // Check if federation exists
        match federations.get_mut(federation_id) {
            Some(fed_state) => {
                // Get peer info
                let peer_info = match self.network.get_peer_info(&peer_id).await {
                    Ok(info) => info,
                    Err(e) => return Err(anyhow::anyhow!("Failed to get peer info: {}", e)),
                };
                
                // Add peer to federation
                fed_state.peers.insert(peer_id, peer_info);
                fed_state.metrics.peer_count = fed_state.peers.len();
                fed_state.metrics.last_sync = Some(std::time::Instant::now());
                
                Ok(())
            },
            None => Err(anyhow::anyhow!("Federation {} does not exist", federation_id)),
        }
    }
    
    /// Stop the network
    pub async fn shutdown(&self) -> Result<()> {
        // Clean up WireGuard interfaces
        if let Some(wg) = &self.wireguard {
            let interface_name = {
                let wg_read = wg.read().await;
                wg_read.interface_name.clone()
            };
            
            // Clean up the WireGuard interface
            let wg_interface = wg.read().await;
            if let Err(e) = wg_interface.cleanup().await {
                println!("Failed to clean up WireGuard interface {}: {}", interface_name, e);
            }
        }
        
        // Clean up federation WireGuard interfaces
        let federations = self.federations.read().await;
        for (fed_id, fed_state) in federations.iter() {
            if let Some(wg) = &fed_state.wireguard {
                let wg_interface = wg.read().await;
                if let Err(e) = wg_interface.cleanup().await {
                    println!("Failed to clean up federation {} WireGuard interface: {}", fed_id, e);
                }
            }
        }
        
        // Stop the network service
        self.network.stop().await
            .map_err(|e| anyhow::anyhow!("Failed to stop network: {}", e))?;
            
        Ok(())
    }
}

// Implement conversion from StorageService to the Storage trait required by network
impl From<StorageService> for Arc<dyn icn_network::Storage + Send + Sync> {
    fn from(storage: StorageService) -> Self {
        // This would need actual implementation to adapt our storage to the network storage trait
        // For now, we're using a placeholder implementation
        Arc::new(MockStorage::new())
    }
}

// Mock storage implementation for the network crate
struct MockStorage {
    // Mock implementation details
}

impl MockStorage {
    fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl icn_network::Storage for MockStorage {
    async fn get(&self, key: &str) -> std::result::Result<Vec<u8>, icn_network::StorageError> {
        // Mock implementation - would be replaced with actual StorageService integration
        Ok(Vec::new())
    }
    
    async fn put(&self, key: &str, value: &[u8]) -> std::result::Result<(), icn_network::StorageError> {
        // Mock implementation
        Ok(())
    }
    
    async fn delete(&self, key: &str) -> std::result::Result<(), icn_network::StorageError> {
        // Mock implementation
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> std::result::Result<bool, icn_network::StorageError> {
        // Mock implementation
        Ok(false)
    }
    
    async fn list_keys(&self, prefix: &str) -> std::result::Result<Vec<String>, icn_network::StorageError> {
        // Mock implementation
        Ok(Vec::new())
    }
} 