//! Overlay network implementation for ICN
//! 
//! This module provides a decentralized overlay network that enables
//! discovery, routing, and communication between ICN nodes across
//! organizational and network boundaries. It uses IPv6 as the overlay
//! protocol with tunneling for secure cross-federation communication.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
    net::{IpAddr, Ipv6Addr, SocketAddr},
    fmt,
};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug, trace};

mod address;
mod routing;
mod dht;
mod onion;

pub use self::address::{OverlayAddress, AddressAllocator, AddressSpace, AddressAllocationStrategy};
pub use self::routing::{RouteManager, RouteInfo, RoutingTable};
pub use self::dht::{DistributedHashTable, NodeInfo, Key, Value};
pub use self::onion::{OnionRouter, Circuit};

use crate::error::{Result, NetworkError};

/// Tunnel type for the overlay network
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TunnelType {
    /// No tunneling, direct IPv6 communication
    Direct,
    /// WireGuard-based tunnel
    WireGuard,
    /// TLS-based tunnel
    Tls,
    /// Onion-routed tunnel for enhanced privacy
    Onion,
}

/// Tunnel state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    /// Tunnel identifier
    pub id: String,
    /// Type of tunnel
    pub tunnel_type: TunnelType,
    /// Remote endpoint of the tunnel
    pub remote_endpoint: SocketAddr,
    /// Local overlay address
    pub local_overlay_addr: OverlayAddress,
    /// Remote overlay address
    pub remote_overlay_addr: OverlayAddress,
    /// MTU of the tunnel
    pub mtu: u16,
    /// Whether the tunnel is active
    pub active: bool,
    /// Last activity timestamp
    pub last_activity: i64,
}

/// Packet forwarding policy
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ForwardingPolicy {
    /// Forward all packets
    ForwardAll,
    /// Forward only packets to known destinations
    ForwardKnown,
    /// Do not forward packets
    NoForwarding,
}

/// The main overlay network manager
pub struct OverlayNetworkManager {
    /// Local node's overlay address
    local_address: Option<OverlayAddress>,
    /// Address allocator for generating new addresses
    address_allocator: AddressAllocator,
    /// Route manager for finding paths in the overlay
    route_manager: RouteManager,
    /// Distributed hash table for discovery and storage
    distributed_hash_table: DistributedHashTable,
    /// Onion routing for privacy
    onion_router: OnionRouter,
    /// Connected peers in the overlay
    peers: Arc<RwLock<HashMap<OverlayAddress, NodeInfo>>>,
    /// Active tunnels
    tunnels: Arc<RwLock<HashMap<String, TunnelInfo>>>,
    /// IPv6 tunnel interfaces
    tun_interfaces: Arc<RwLock<HashMap<String, String>>>,
    /// Packet forwarding policy
    forwarding_policy: ForwardingPolicy,
    /// Channel for sending packets
    packet_tx: Option<mpsc::Sender<(OverlayAddress, Vec<u8>)>>,
    /// Channel for receiving packets
    packet_rx: Option<mpsc::Receiver<(OverlayAddress, Vec<u8>)>>,
}

/// Options for sending data through the overlay network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverlayOptions {
    /// Whether anonymity is required
    pub anonymity_required: bool,
    /// Whether delivery must be reliable
    pub reliability_required: bool,
    /// Message priority level
    pub priority: MessagePriority,
    /// Tunnel type to use (if None, use default)
    pub tunnel_type: Option<TunnelType>,
    /// Time-to-live for the packet
    pub ttl: u8,
}

/// Priority levels for overlay messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessagePriority {
    Low,
    Normal,
    High, 
    Critical,
}

impl Default for OverlayOptions {
    fn default() -> Self {
        Self {
            anonymity_required: false,
            reliability_required: true,
            priority: MessagePriority::Normal,
            tunnel_type: None,
            ttl: 64,
        }
    }
}

/// IPv6 packet for the overlay network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6Packet {
    /// Source address
    pub source: OverlayAddress,
    /// Destination address
    pub destination: OverlayAddress,
    /// Next header (protocol)
    pub next_header: u8,
    /// Hop limit (TTL)
    pub hop_limit: u8,
    /// Traffic class
    pub traffic_class: u8,
    /// Flow label
    pub flow_label: u32,
    /// Payload data
    pub payload: Vec<u8>,
}

impl Ipv6Packet {
    /// Create a new IPv6 packet
    pub fn new(
        source: OverlayAddress,
        destination: OverlayAddress,
        next_header: u8,
        payload: Vec<u8>,
        options: &OverlayOptions,
    ) -> Self {
        let traffic_class = match options.priority {
            MessagePriority::Low => 0,
            MessagePriority::Normal => 8,  // CS0
            MessagePriority::High => 40,   // CS5
            MessagePriority::Critical => 48, // CS6
        };
        
        Self {
            source,
            destination,
            next_header,
            hop_limit: options.ttl,
            traffic_class,
            flow_label: 0, // Could generate based on flow if needed
            payload,
        }
    }
    
    /// Serialize the packet to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).map_err(|e| NetworkError::Other(format!("Serialization error: {}", e)))
    }
    
    /// Deserialize bytes to a packet
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).map_err(|e| NetworkError::Other(format!("Deserialization error: {}", e)))
    }
}

impl fmt::Display for Ipv6Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "IPv6: {} -> {}, proto: {}, hop_limit: {}, len: {}",
            self.source, self.destination, self.next_header, 
            self.hop_limit, self.payload.len())
    }
}

#[async_trait]
pub trait OverlayNetworkService {
    /// Initialize the overlay network
    async fn initialize(&mut self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress>;
    
    /// Connect to the overlay network
    async fn connect(&mut self, bootstrap_nodes: &[OverlayAddress]) -> Result<()>;
    
    /// Send data through the overlay
    async fn send_data(&self, destination: &OverlayAddress, data: &[u8], options: &OverlayOptions) -> Result<()>;
    
    /// Receive data from the overlay
    async fn receive_data(&self) -> Result<(OverlayAddress, Vec<u8>)>;
    
    /// Get the local overlay address
    fn get_local_address(&self) -> Option<OverlayAddress>;
    
    /// Find a route to a destination
    async fn find_route(&self, destination: &OverlayAddress) -> Result<RouteInfo>;
    
    /// Get known peers in the overlay
    fn get_peers(&self) -> Result<Vec<NodeInfo>>;
    
    /// Create a tunnel to a remote node
    async fn create_tunnel(&mut self, remote_addr: &OverlayAddress, tunnel_type: TunnelType) -> Result<TunnelInfo>;
    
    /// Close a tunnel
    async fn close_tunnel(&mut self, tunnel_id: &str) -> Result<()>;
    
    /// Get active tunnels
    fn get_tunnels(&self) -> Result<Vec<TunnelInfo>>;
    
    /// Set the packet forwarding policy
    fn set_forwarding_policy(&mut self, policy: ForwardingPolicy) -> Result<()>;
}

impl OverlayNetworkManager {
    /// Create a new overlay network manager
    pub fn new() -> Self {
        let (packet_tx, packet_rx) = mpsc::channel(1000);
        
        Self {
            local_address: None,
            address_allocator: AddressAllocator::new(),
            route_manager: RouteManager::new(),
            distributed_hash_table: DistributedHashTable::new(),
            onion_router: OnionRouter::new(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            tun_interfaces: Arc::new(RwLock::new(HashMap::new())),
            forwarding_policy: ForwardingPolicy::ForwardKnown,
            packet_tx: Some(packet_tx),
            packet_rx: Some(packet_rx),
        }
    }
    
    /// Create a new overlay network manager with custom address allocator
    pub fn with_address_allocator(address_allocator: AddressAllocator) -> Self {
        let (packet_tx, packet_rx) = mpsc::channel(1000);
        
        Self {
            local_address: None,
            address_allocator,
            route_manager: RouteManager::new(),
            distributed_hash_table: DistributedHashTable::new(),
            onion_router: OnionRouter::new(),
            peers: Arc::new(RwLock::new(HashMap::new())),
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            tun_interfaces: Arc::new(RwLock::new(HashMap::new())),
            forwarding_policy: ForwardingPolicy::ForwardKnown,
            packet_tx: Some(packet_tx),
            packet_rx: Some(packet_rx),
        }
    }
    
    /// Create a TUN interface for IPv6 overlay routing
    async fn create_tun_interface(&self, name: &str, address: &OverlayAddress) -> Result<String> {
        // In a real implementation, this would create an actual TUN interface
        // with the appropriate IPv6 address and routing table
        
        info!("Would create TUN interface {} with address {}", name, address);
        
        // Simulated interface name
        let interface_name = format!("tun-icn-{}", name);
        
        let mut interfaces = self.tun_interfaces.write().map_err(|_| NetworkError::LockError)?;
        interfaces.insert(name.to_string(), interface_name.clone());
        
        Ok(interface_name)
    }
    
    /// Setup packet forwarding between tunnels
    async fn setup_packet_forwarding(&self) -> Result<()> {
        match self.forwarding_policy {
            ForwardingPolicy::ForwardAll => {
                info!("Setting up forwarding for all packets");
                // In a real implementation, this would enable IPv6 forwarding
            },
            ForwardingPolicy::ForwardKnown => {
                info!("Setting up forwarding only for known destinations");
                // In a real implementation, this would set up selective forwarding
            },
            ForwardingPolicy::NoForwarding => {
                info!("Disabling packet forwarding");
                // In a real implementation, this would disable forwarding
            },
        }
        
        Ok(())
    }
    
    /// Forward an IPv6 packet according to the routing table
    async fn forward_packet(&self, packet: &Ipv6Packet) -> Result<()> {
        // Decrement hop limit
        let mut forwarded_packet = packet.clone();
        forwarded_packet.hop_limit -= 1;
        
        // Check if packet has expired
        if forwarded_packet.hop_limit == 0 {
            return Err(NetworkError::Other("Packet hop limit expired".into()));
        }
        
        // Find next hop
        let route = self.find_route(&packet.destination).await?;
        
        // Check forwarding policy
        match self.forwarding_policy {
            ForwardingPolicy::NoForwarding => {
                return Err(NetworkError::Other("Forwarding disabled".into()));
            },
            ForwardingPolicy::ForwardKnown => {
                if self.route_manager.find_route(&packet.destination).is_err() {
                    return Err(NetworkError::Other("Unknown destination".into()));
                }
            },
            ForwardingPolicy::ForwardAll => {
                // Continue forwarding
            }
        }
        
        if let Some(next_hop) = route.next_hop {
            // Forward to next hop
            if let Some(tx) = &self.packet_tx {
                let data = forwarded_packet.to_bytes()?;
                tx.send((next_hop, data)).await.map_err(|_| NetworkError::Other("Channel closed".into()))?;
            }
            
            trace!("Forwarded packet: {} via {}", forwarded_packet, next_hop);
            Ok(())
        } else {
            Err(NetworkError::Other("No route to destination".into()))
        }
    }
}

#[async_trait]
impl OverlayNetworkService for OverlayNetworkManager {
    async fn initialize(&mut self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress> {
        // Allocate an overlay address
        let address = self.address_allocator.allocate_address(node_id, federation_id)?;
        
        // Initialize routing table
        self.route_manager.initialize(&address)?;
        
        // Initialize DHT
        self.distributed_hash_table.initialize(node_id, &address)?;
        
        // Initialize onion router
        self.onion_router.initialize()?;
        
        // Store local address
        self.local_address = Some(address.clone());
        
        // Create TUN interface for IPv6 overlay
        self.create_tun_interface("main", &address).await?;
        
        // Setup packet forwarding
        self.setup_packet_forwarding().await?;
        
        info!("Initialized overlay network with address: {}", address);
        Ok(address)
    }
    
    async fn connect(&mut self, bootstrap_nodes: &[OverlayAddress]) -> Result<()> {
        info!("Connecting to overlay network with {} bootstrap nodes", bootstrap_nodes.len());
        
        // Try to connect to bootstrap nodes
        for address in bootstrap_nodes {
            debug!("Connecting to bootstrap node: {}", address);
            
            // Create a tunnel to the bootstrap node
            let tunnel_type = TunnelType::Direct; // Default to direct for bootstrap
            let _ = self.create_tunnel(address, tunnel_type).await?;
            
            // In a real implementation, establish connection and exchange routing information
            // For now, just add to peers
            if let Some(local_addr) = &self.local_address {
                let node_info = NodeInfo {
                    id: format!("bootstrap-{}", address),
                    address: address.clone(),
                    last_seen: chrono::Utc::now().timestamp(),
                };
                
                let mut peers = self.peers.write().map_err(|_| NetworkError::LockError)?;
                peers.insert(address.clone(), node_info);
                
                // Add a route to the bootstrap node
                self.route_manager.add_route(RouteInfo {
                    destination: address.clone(),
                    next_hop: Some(address.clone()),
                    path: vec![local_addr.clone(), address.clone()],
                    cost: 1,
                    last_updated: chrono::Utc::now().timestamp(),
                })?;
            }
            
            // Register with DHT
            if let Some(local_addr) = &self.local_address {
                self.distributed_hash_table.register(local_addr).await?;
            }
        }
        
        info!("Connected to overlay network");
        Ok(())
    }
    
    async fn send_data(&self, destination: &OverlayAddress, data: &[u8], options: &OverlayOptions) -> Result<()> {
        // Ensure we have a local address
        let source = self.local_address.as_ref().ok_or_else(|| 
            NetworkError::Other("Local address not initialized".into())
        )?;
        
        // Create IPv6 packet
        let next_header = 6; // TCP by default, could be configurable
        let packet = Ipv6Packet::new(
            source.clone(),
            destination.clone(),
            next_header,
            data.to_vec(),
            options
        );
        
        // Find route to destination
        let route = self.find_route(destination).await?;
        
        if options.anonymity_required {
            // Use onion routing for anonymity
            let circuit = self.onion_router.get_or_create_circuit(destination)?;
            trace!("Sending data via onion circuit: {:?}", circuit);
            
            let packet_data = packet.to_bytes()?;
            self.onion_router.send_through_circuit(&circuit, destination, &packet_data)?;
        } else {
            // Select tunnel type
            let tunnel_type = options.tunnel_type.unwrap_or_else(|| {
                if let Some(federation) = &destination.federation {
                    if let Some(local_fed) = source.federation.as_ref() {
                        if federation == local_fed {
                            // Same federation, use direct
                            TunnelType::Direct
                        } else {
                            // Different federation, use WireGuard
                            TunnelType::WireGuard
                        }
                    } else {
                        // Local not in federation, remote is - use WireGuard
                        TunnelType::WireGuard
                    }
                } else {
                    // No federations involved, use direct
                    TunnelType::Direct
                }
            });
            
            // Use regular routing
            if let Some(next_hop) = &route.next_hop {
                debug!("Sending data to {} via next hop {}", destination, next_hop);
                
                // Find or create tunnel to next hop
                let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
                let tunnel = tunnels.values().find(|t| 
                    t.remote_overlay_addr == *next_hop && 
                    t.tunnel_type == tunnel_type && 
                    t.active
                );
                
                drop(tunnels);
                
                if let Some(tunnel) = tunnel {
                    // Send through existing tunnel
                    let packet_data = packet.to_bytes()?;
                    trace!("Sending packet through tunnel {}: {}", tunnel.id, packet);
                    
                    // In a real implementation, write to TUN interface
                    if let Some(tx) = &self.packet_tx {
                        tx.send((next_hop.clone(), packet_data)).await
                           .map_err(|_| NetworkError::Other("Channel closed".into()))?;
                    }
                } else {
                    // Need to create a new tunnel
                    debug!("Creating new tunnel to {}", next_hop);
                    drop(tunnels);
                    
                    // In a real implementation, this would be done asynchronously
                    // and packet would be queued until tunnel is established
                    return Err(NetworkError::Other(format!("No tunnel to {}", next_hop)));
                }
            } else {
                // Direct delivery
                debug!("Direct delivery to {}", destination);
                // In a real implementation, deliver directly
                
                // Send to packet channel
                let packet_data = packet.to_bytes()?;
                if let Some(tx) = &self.packet_tx {
                    tx.send((destination.clone(), packet_data)).await
                        .map_err(|_| NetworkError::Other("Channel closed".into()))?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn receive_data(&self) -> Result<(OverlayAddress, Vec<u8>)> {
        // Try to receive from the channel
        if let Some(mut rx) = self.packet_rx.clone() {
            if let Some((source, data)) = rx.recv().await {
                // In a real implementation, this would read from TUN interface
                // and process the IPv6 packet
                
                // Parse the packet
                let packet = Ipv6Packet::from_bytes(&data)?;
                
                // Check if packet is for us
                if let Some(local_addr) = &self.local_address {
                    if packet.destination == *local_addr {
                        trace!("Received packet for local address: {}", packet);
                        return Ok((source, packet.payload));
                    } else {
                        // Forward the packet
                        self.forward_packet(&packet).await?;
                        return Err(NetworkError::Other("Packet forwarded".into()));
                    }
                }
                
                // Return raw data if parsing fails or not for us
                return Ok((source, data));
            }
        }
        
        Err(NetworkError::Other("No data available".into()))
    }
    
    fn get_local_address(&self) -> Option<OverlayAddress> {
        self.local_address.clone()
    }
    
    async fn find_route(&self, destination: &OverlayAddress) -> Result<RouteInfo> {
        self.route_manager.find_route(destination)
    }
    
    fn get_peers(&self) -> Result<Vec<NodeInfo>> {
        let peers = self.peers.read().map_err(|_| NetworkError::LockError)?;
        Ok(peers.values().cloned().collect())
    }
    
    async fn create_tunnel(&mut self, remote_addr: &OverlayAddress, tunnel_type: TunnelType) -> Result<TunnelInfo> {
        let local_addr = self.local_address.as_ref().ok_or_else(|| 
            NetworkError::Other("Local address not initialized".into())
        )?;
        
        // Check if tunnel already exists
        let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
        if let Some(tunnel) = tunnels.values().find(|t| 
            t.remote_overlay_addr == *remote_addr && 
            t.tunnel_type == tunnel_type && 
            t.active
        ) {
            return Ok(tunnel.clone());
        }
        drop(tunnels);
        
        // Generate a tunnel ID
        let tunnel_id = format!("tunnel-{}-{}-{}", 
            local_addr.to_ipv6().segments()[7],
            remote_addr.to_ipv6().segments()[7],
            chrono::Utc::now().timestamp() % 1000
        );
        
        // Create tunnel info
        let remote_endpoint = SocketAddr::new(
            IpAddr::V6(remote_addr.to_ipv6()),
            4789 // Default VXLAN port, could be configured
        );
        
        let tunnel_info = TunnelInfo {
            id: tunnel_id.clone(),
            tunnel_type,
            remote_endpoint,
            local_overlay_addr: local_addr.clone(),
            remote_overlay_addr: remote_addr.clone(),
            mtu: 1420, // WireGuard default MTU
            active: true,
            last_activity: chrono::Utc::now().timestamp(),
        };
        
        // In a real implementation, set up the actual tunnel
        info!("Creating {} tunnel to {}", 
            match tunnel_type {
                TunnelType::Direct => "direct",
                TunnelType::WireGuard => "WireGuard",
                TunnelType::Tls => "TLS",
                TunnelType::Onion => "onion-routed",
            },
            remote_addr
        );
        
        // Store the tunnel info
        let mut tunnels = self.tunnels.write().map_err(|_| NetworkError::LockError)?;
        tunnels.insert(tunnel_id, tunnel_info.clone());
        
        Ok(tunnel_info)
    }
    
    async fn close_tunnel(&mut self, tunnel_id: &str) -> Result<()> {
        // Get the tunnel
        let tunnel = {
            let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
            tunnels.get(tunnel_id).cloned()
        };
        
        if let Some(mut tunnel_info) = tunnel {
            // Set tunnel as inactive
            tunnel_info.active = false;
            
            // Update the tunnel info
            let mut tunnels = self.tunnels.write().map_err(|_| NetworkError::LockError)?;
            tunnels.insert(tunnel_id.to_string(), tunnel_info.clone());
            
            // In a real implementation, tear down the actual tunnel
            info!("Closed tunnel {} to {}", tunnel_id, tunnel_info.remote_overlay_addr);
            
            Ok(())
        } else {
            Err(NetworkError::Other(format!("Tunnel {} not found", tunnel_id)))
        }
    }
    
    fn get_tunnels(&self) -> Result<Vec<TunnelInfo>> {
        let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
        Ok(tunnels.values().cloned().collect())
    }
    
    fn set_forwarding_policy(&mut self, policy: ForwardingPolicy) -> Result<()> {
        self.forwarding_policy = policy;
        
        // In a real implementation, update system forwarding settings
        info!("Set forwarding policy to {:?}", policy);
        
        Ok(())
    }
}
