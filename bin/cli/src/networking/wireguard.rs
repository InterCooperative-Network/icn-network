use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use anyhow::Result;
use ipnetwork::IpNetwork;
use libp2p::PeerId;
use rand::{Rng, rngs::OsRng};
use tokio::sync::RwLock;
use wireguard_control::{Backend, Device, DeviceUpdate, InterfaceName, Key, KeyPair, PeerConfig};

/// Information about a WireGuard peer
pub struct WireGuardPeer {
    /// Peer ID in the libp2p network
    pub peer_id: PeerId,
    /// WireGuard public key
    pub public_key: Key,
    /// Endpoint address
    pub endpoint: SocketAddr,
    /// Allowed IP networks
    pub allowed_ips: Vec<IpNetwork>,
    /// Last handshake time
    pub last_handshake: Option<u64>,
    /// Bytes received
    pub rx_bytes: u64,
    /// Bytes transmitted
    pub tx_bytes: u64,
}

/// Create secure overlay using WireGuard
pub struct WireGuardOverlay {
    /// Interface name
    pub interface_name: String,
    /// Private key
    private_key: Key,
    /// Public key
    public_key: Key,
    /// Connected peers
    peers: HashMap<PeerId, WireGuardPeer>,
    /// Listen port
    listen_port: u16,
    /// Local IP address
    local_ip: IpAddr,
}

impl WireGuardOverlay {
    /// Create a new WireGuard overlay
    pub async fn new(interface_name: &str, listen_port: u16) -> Result<Self> {
        // Generate keypair
        let keypair = KeyPair::generate();
        
        // Get interface name
        let interface = InterfaceName::from_string(interface_name.to_string())?;
        
        // Generate a unique local IP address in the 10.0.0.0/8 range
        let mut rng = OsRng;
        let octet2 = rng.gen::<u8>();
        let octet3 = rng.gen::<u8>();
        let local_ip = IpAddr::V4(Ipv4Addr::new(10, octet2, octet3, 1));
        
        // Setup WireGuard interface
        let device = DeviceUpdate::new()
            .set_key(keypair.private)
            .set_listen_port(listen_port);
        
        let backend = Backend::default();
        
        // First check if the interface already exists and remove it
        if backend.device_list()?.contains(&interface) {
            backend.delete_device(&interface)?;
        }
        
        // Create new interface
        backend.set_device(&interface, device)?;
        
        // Configure IP address (this would use OS-specific commands)
        // For now we'll simulate this

        Ok(Self {
            interface_name: interface_name.to_string(),
            private_key: keypair.private,
            public_key: keypair.public,
            peers: HashMap::new(),
            listen_port,
            local_ip,
        })
    }
    
    /// Add a peer to the WireGuard interface
    pub async fn add_peer(&mut self, peer_id: PeerId, endpoint: SocketAddr, allowed_ips: Vec<IpNetwork>) -> Result<String> {
        // Generate key for the peer
        let peer_keypair = KeyPair::generate();
        
        // Determine allowed IPs if none provided
        let allowed_ips = if allowed_ips.is_empty() {
            // Generate a /24 network in 10.0.0.0/8 range that doesn't conflict with local IP
            let mut rng = OsRng;
            let octet2 = rng.gen::<u8>();
            let octet3 = rng.gen::<u8>();
            
            // Avoid collision with local IP
            let (octet2, octet3) = if octet2 == self.local_ip.to_string().split('.').nth(1).unwrap_or("0").parse::<u8>().unwrap_or(0) &&
               octet3 == self.local_ip.to_string().split('.').nth(2).unwrap_or("0").parse::<u8>().unwrap_or(0) {
                // Regenerate if collision
                (rng.gen::<u8>(), rng.gen::<u8>())
            } else {
                (octet2, octet3)
            };
            
            let network = format!("10.{}.{}.0/24", octet2, octet3);
            vec![network.parse::<IpNetwork>()?]
        } else {
            allowed_ips
        };
        
        // Get interface
        let interface = InterfaceName::from_string(self.interface_name.clone())?;
        
        // Create peer configuration
        let peer_config = PeerConfig::new()
            .set_public_key(peer_keypair.public)
            .set_endpoint(endpoint)
            .set_allowed_ips(allowed_ips.clone());
        
        // Update WireGuard configuration
        let device_update = DeviceUpdate::new().add_peer(peer_config);
        Backend::default().set_device(&interface, device_update)?;
        
        // Store peer information
        let peer = WireGuardPeer {
            peer_id: peer_id.clone(),
            public_key: peer_keypair.public,
            endpoint,
            allowed_ips: allowed_ips.clone(),
            last_handshake: None,
            rx_bytes: 0,
            tx_bytes: 0,
        };
        
        self.peers.insert(peer_id, peer);
        
        // Return the interface name as the tunnel ID
        Ok(self.interface_name.clone())
    }
    
    /// Remove a peer from the WireGuard interface
    pub async fn remove_peer(&mut self, peer_id: &PeerId) -> Result<()> {
        // Get the peer
        let peer = match self.peers.remove(peer_id) {
            Some(peer) => peer,
            None => return Ok(()), // Peer doesn't exist, nothing to do
        };
        
        // Get interface
        let interface = InterfaceName::from_string(self.interface_name.clone())?;
        
        // Create peer configuration to remove
        let peer_config = PeerConfig::new()
            .set_public_key(peer.public_key)
            .set_remove(true);
        
        // Update WireGuard configuration
        let device_update = DeviceUpdate::new().add_peer(peer_config);
        Backend::default().set_device(&interface, device_update)?;
        
        Ok(())
    }
    
    /// Get peer information
    pub async fn get_peer(&self, peer_id: &PeerId) -> Option<&WireGuardPeer> {
        self.peers.get(peer_id)
    }
    
    /// Get all peers
    pub async fn get_peers(&self) -> Vec<&WireGuardPeer> {
        self.peers.values().collect()
    }
    
    /// Update peer stats
    pub async fn update_stats(&mut self) -> Result<()> {
        // Get interface
        let interface = InterfaceName::from_string(self.interface_name.clone())?;
        
        // Get device information
        let device = Backend::default().get_device(&interface)?;
        
        // Update peer stats
        for peer in device.peers {
            let peer_key = peer.config.public_key;
            
            // Find the peer in our list
            if let Some(peer_id) = self.peers.iter().find_map(|(id, p)| {
                if p.public_key == peer_key {
                    Some(id.clone())
                } else {
                    None
                }
            }) {
                if let Some(peer_info) = self.peers.get_mut(&peer_id) {
                    // Update stats
                    peer_info.last_handshake = peer.stats.last_handshake_time;
                    peer_info.rx_bytes = peer.stats.rx_bytes;
                    peer_info.tx_bytes = peer.stats.tx_bytes;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get WireGuard interface information
    pub async fn get_interface_info(&self) -> Result<Device> {
        // Get interface
        let interface = InterfaceName::from_string(self.interface_name.clone())?;
        
        // Get device information
        Ok(Backend::default().get_device(&interface)?)
    }
    
    /// Clean up the WireGuard interface
    pub async fn cleanup(&self) -> Result<()> {
        // Get interface
        let interface = InterfaceName::from_string(self.interface_name.clone())?;
        
        // Delete the interface
        Backend::default().delete_device(&interface)?;
        
        Ok(())
    }
} 