use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
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
        
        Ok(Self {
            storage,
            network: Arc::new(network),
            connections: Arc::new(RwLock::new(Vec::new())),
            wireguard: None,
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
    
    /// Stop the network
    pub async fn shutdown(&self) -> Result<()> {
        // Clean up WireGuard interfaces
        if let Some(wg) = &self.wireguard {
            let interface_name = {
                let wg_read = wg.read().await;
                wg_read.interface_name.clone()
            };
            
            // Remove the WireGuard interface
            // This would use wireguard_control to clean up
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