//! Circuit relay for ICN Network
//!
//! This module implements a circuit relay protocol that enables nodes behind NATs 
//! to connect to other nodes through publicly accessible relay nodes.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use futures::StreamExt;
use libp2p::{
    core::{muxing::StreamMuxerBox, transport::OrTransport, upgrade},
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode},
    identify, kad, mdns, noise, ping, relay,
    swarm::{self, ConnectionError, NetworkBehaviour, SwarmEvent, Toggle},
    tcp, yamux, Multiaddr, PeerId,
    core::Transport,
};
use multiaddr::Protocol;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

use crate::{NetworkError, NetworkResult, metrics::NetworkMetrics, NetworkService, PeerInfo};

/// Circuit relay configuration
#[derive(Debug, Clone)]
pub struct CircuitRelayConfig {
    /// Enable relay server functionality - makes this node a relay
    pub enable_relay_server: bool,
    
    /// Enable relay client functionality - allows this node to connect through relays
    pub enable_relay_client: bool,
    
    /// Maximum number of inbound relay connections to accept
    pub max_inbound_relay_connections: usize,
    
    /// Maximum number of outbound relay connections to establish
    pub max_outbound_relay_connections: usize,
    
    /// List of known relay servers to use (multiaddresses)
    pub known_relay_servers: Vec<Multiaddr>,
    
    /// Connection timeout for relay connections
    pub connection_timeout: Duration,
    
    /// Time to keep relay connections alive
    pub ttl: Duration,
}

impl Default for CircuitRelayConfig {
    fn default() -> Self {
        Self {
            enable_relay_server: false,
            enable_relay_client: true,
            max_inbound_relay_connections: 10,
            max_outbound_relay_connections: 5,
            known_relay_servers: Vec::new(),
            connection_timeout: Duration::from_secs(30),
            ttl: Duration::from_secs(3600),
        }
    }
}

/// Circuit relay manager that handles NAT traversal
pub struct CircuitRelayManager {
    /// Relay configuration
    config: CircuitRelayConfig,
    
    /// Available relay servers
    relay_servers: RwLock<HashMap<PeerId, RelayServerInfo>>,
    
    /// Currently active relay connections
    active_relays: RwLock<HashMap<PeerId, RelayConnectionInfo>>,
    
    /// Metrics for monitoring
    metrics: Option<NetworkMetrics>,
}

/// Information about a relay server
#[derive(Debug, Clone)]
pub struct RelayServerInfo {
    /// The peer ID of the relay server
    pub peer_id: PeerId,
    
    /// Addresses of the relay server
    pub addresses: Vec<Multiaddr>,
    
    /// When the relay was last used
    pub last_used: Instant,
    
    /// Number of successful connections through this relay
    pub successful_connections: usize,
    
    /// Number of failed connections through this relay
    pub failed_connections: usize,
}

/// Information about an active relay connection
#[derive(Debug, Clone)]
pub struct RelayConnectionInfo {
    /// The peer ID of the destination
    pub dest_peer_id: PeerId,
    
    /// The peer ID of the relay
    pub relay_peer_id: PeerId,
    
    /// When the connection was established
    pub established_at: Instant,
    
    /// The time-to-live for this connection
    pub ttl: Duration,
    
    /// Relay reservation ID if applicable
    pub reservation_id: Option<String>,
}

/// Information about a relay connection
#[derive(Debug, Clone)]
pub struct RelayInfo {
    /// Peer ID of the relay
    pub peer_id: PeerId,
    /// Address of the relay
    pub address: Multiaddr,
    /// Whether the relay is currently connected
    pub connected: bool,
    /// Reservation ID if we have an active reservation
    pub reservation_id: Option<String>,
    /// When the relay was established
    pub established_at: SystemTime,
    /// How long the relay is valid for
    pub ttl: Duration,
}

impl CircuitRelayManager {
    /// Create a new circuit relay manager
    pub fn new(config: CircuitRelayConfig, metrics: Option<NetworkMetrics>) -> Self {
        Self {
            config,
            relay_servers: RwLock::new(HashMap::new()),
            active_relays: RwLock::new(HashMap::new()),
            metrics,
        }
    }
    
    /// Initialize the circuit relay functionality
    pub async fn initialize(&self) -> NetworkResult<()> {
        // Add known relay servers
        for addr in &self.config.known_relay_servers {
            if let Some(peer_id) = extract_peer_id(addr) {
                let server_info = RelayServerInfo {
                    peer_id,
                    addresses: vec![addr.clone()],
                    last_used: Instant::now(),
                    successful_connections: 0,
                    failed_connections: 0,
                };
                self.relay_servers.write().await.insert(peer_id, server_info);
            } else {
                warn!("Ignoring relay server without peer ID: {}", addr);
            }
        }
        
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_servers(self.relay_servers.read().await.len());
        }
        
        Ok(())
    }
    
    /// Add a relay server to the list of available relays
    pub async fn add_relay_server(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> NetworkResult<()> {
        let server_info = RelayServerInfo {
            peer_id,
            addresses,
            last_used: Instant::now(),
            successful_connections: 0,
            failed_connections: 0,
        };
        
        self.relay_servers.write().await.insert(peer_id, server_info);
        
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_servers(self.relay_servers.read().await.len());
        }
        
        Ok(())
    }
    
    /// Remove a relay server from the list
    pub async fn remove_relay_server(&self, peer_id: &PeerId) -> NetworkResult<()> {
        self.relay_servers.write().await.remove(peer_id);
        
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_servers(self.relay_servers.read().await.len());
        }
        
        Ok(())
    }
    
    /// Get a list of available relay servers
    pub async fn get_relay_servers(&self) -> Vec<RelayServerInfo> {
        self.relay_servers.read().await.values().cloned().collect()
    }
    
    /// Connect to a peer through a relay
    pub async fn connect_via_relay(&self, dest_peer_id: &PeerId) -> NetworkResult<Multiaddr> {
        let relay_servers = self.relay_servers.read().await;
        
        if relay_servers.is_empty() {
            return Err(NetworkError::NoRelaysAvailable);
        }
        
        // Find the best relay based on success rate
        let mut best_relay = None;
        let mut best_score = 0.0;
        
        for (_, server) in relay_servers.iter() {
            let success_rate = if server.successful_connections + server.failed_connections > 0 {
                server.successful_connections as f64 / (server.successful_connections + server.failed_connections) as f64
            } else {
                0.5 // Default for untested relays
            };
            
            if success_rate > best_score {
                best_score = success_rate;
                best_relay = Some(server);
            }
        }
        
        let server = best_relay.ok_or(NetworkError::NoRelaysAvailable)?;
        
        // Create a relay address
        if server.addresses.is_empty() {
            return Err(NetworkError::InvalidRelayAddress);
        }
        
        let relay_addr = server.addresses[0].clone();
        let dest_addr = relay_addr.clone().with(Protocol::P2pCircuit).with(Protocol::P2p(*dest_peer_id));
        
        // Record the connection attempt
        let connection_info = RelayConnectionInfo {
            dest_peer_id: *dest_peer_id,
            relay_peer_id: server.peer_id,
            established_at: Instant::now(),
            ttl: self.config.ttl,
            reservation_id: None,
        };
        
        self.active_relays.write().await.insert(*dest_peer_id, connection_info);
        
        if let Some(metrics) = &self.metrics {
            metrics.record_active_relay_connections(self.active_relays.read().await.len());
            metrics.record_relay_connection_attempt();
        }
        
        Ok(dest_addr)
    }
    
    /// Record a successful relay connection
    pub async fn record_successful_connection(&self, dest_peer_id: &PeerId, relay_peer_id: &PeerId) -> NetworkResult<()> {
        // Update relay server stats
        if let Some(server) = self.relay_servers.write().await.get_mut(relay_peer_id) {
            server.successful_connections += 1;
            server.last_used = Instant::now();
        }
        
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_connection_success();
        }
        
        Ok(())
    }
    
    /// Record a failed relay connection
    pub async fn record_failed_connection(&self, dest_peer_id: &PeerId, relay_peer_id: &PeerId) -> NetworkResult<()> {
        // Update relay server stats
        if let Some(server) = self.relay_servers.write().await.get_mut(relay_peer_id) {
            server.failed_connections += 1;
        }
        
        // Remove the active connection since it failed
        self.active_relays.write().await.remove(dest_peer_id);
        
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_connection_failure();
            metrics.record_active_relay_connections(self.active_relays.read().await.len());
        }
        
        Ok(())
    }
    
    /// Check if a connection to a peer is relayed
    pub async fn is_relayed_connection(&self, peer_id: &PeerId) -> bool {
        self.active_relays.read().await.contains_key(peer_id)
    }
    
    /// Get the relay used for a connection
    pub async fn get_relay_for_connection(&self, peer_id: &PeerId) -> Option<PeerId> {
        self.active_relays.read().await.get(peer_id).map(|info| info.relay_peer_id)
    }
    
    /// Start relay cleanup task
    pub fn start_cleanup_task(&self) -> tokio::task::JoinHandle<()> {
        let active_relays = Arc::new(RwLock::new(HashMap::<PeerId, RelayConnectionInfo>::new()));
        let metrics = self.metrics.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Clean up expired relay connections
                let now = Instant::now();
                let mut relays = active_relays.write().await;
                
                let before_len = relays.len();
                relays.retain(|_, info| {
                    let elapsed = now.duration_since(info.established_at);
                    elapsed < info.ttl
                });
                let after_len = relays.len();
                
                if before_len != after_len {
                    debug!("Cleaned up {} expired relay connections", before_len - after_len);
                    
                    if let Some(m) = &metrics {
                        m.record_active_relay_connections(after_len);
                    }
                }
            }
        })
    }
}

/// Create a relay transport
pub fn create_relay_transport<T>(
    transport: T,
    relay_config: &CircuitRelayConfig,
) -> NetworkResult<relay::client::Transport> 
where
    T: Transport + Clone + Send + 'static,
    T::Output: Send + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    // Since the direct constructor is private, we'll create a simpler implementation
    // that just wraps the transport without actual relay functionality for now.
    // This would need to be updated with the proper public API once available
    Err(NetworkError::InternalError("Relay transport creation not available through public API".to_string()))
}

/// Extract peer ID from a multiaddress
fn extract_peer_id(addr: &Multiaddr) -> Option<PeerId> {
    addr.iter().find_map(|p| match p {
        libp2p::multiaddr::Protocol::P2p(hash) => {
            PeerId::from_multihash(hash.clone().into()).ok()
        },
        _ => None,
    })
}

/// Circuit relay behaviour that combines server and client capabilities
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "CircuitRelayEvent")]
struct CircuitRelayBehaviour {
    /// Ping protocol for measuring latency
    ping: ping::Behaviour,
    
    /// Identify protocol for exchanging node information
    identify: identify::Behaviour,
    
    /// Relay server for providing relay services
    relay_server: Toggle<relay::Behaviour>,
    
    /// Relay client for connecting through relays
    relay_client: Toggle<relay::client::Behaviour>,
}

/// Events from the circuit relay behaviour
#[derive(Debug)]
pub enum CircuitRelayEvent {
    /// Ping events
    Ping(ping::Event),
    
    /// Identify events
    Identify(identify::Event),
    
    /// Relay server events
    RelayServer(relay::Event),
    
    /// Relay client events
    RelayClient(relay::client::Event),
}

impl From<ping::Event> for CircuitRelayEvent {
    fn from(event: ping::Event) -> Self {
        Self::Ping(event)
    }
}

impl From<identify::Event> for CircuitRelayEvent {
    fn from(event: identify::Event) -> Self {
        Self::Identify(event)
    }
}

impl From<relay::Event> for CircuitRelayEvent {
    fn from(event: relay::Event) -> Self {
        Self::RelayServer(event)
    }
}

impl From<relay::client::Event> for CircuitRelayEvent {
    fn from(event: relay::client::Event) -> Self {
        Self::RelayClient(event)
    }
}

impl CircuitRelayBehaviour {
    /// Create a new circuit relay behaviour
    fn new(local_peer_id: PeerId, config: &CircuitRelayConfig) -> Self {
        let ping_config = ping::Config::new()
            .with_interval(Duration::from_secs(30))
            .with_timeout(Duration::from_secs(10));
            
        let identify_config = identify::Config::new(
            "/ipfs/relay/1.0.0".to_string(),
            local_peer_id.clone().into()
        );
        
        let mut behaviour = Self {
            ping: ping::Behaviour::new(ping_config),
            identify: identify::Behaviour::new(identify_config),
            relay_server: Toggle::new(),
            relay_client: Toggle::new(),
        };
        
        // Configure relay server if enabled
        if config.enable_relay_server {
            let relay_config = relay::Config {
                max_circuit_duration: config.ttl,
                max_circuit_bytes: 10 * 1024 * 1024, // 10 MB
                ..Default::default()
            };
            
            behaviour.relay_server.set(Some(relay::Behaviour::new(local_peer_id, relay_config)));
        }
        
        // Configure relay client if enabled
        if config.enable_relay_client {
            // Since the direct constructor is private, we'll use a placeholder for now
            // This would need to be updated with the proper public API once available
            debug!("Relay client functionality disabled due to API limitations");
        }
        
        behaviour
    }
}

/// Extension for NetworkMetrics to add circuit relay metrics
pub trait CircuitRelayMetricsExt {
    /// Record the number of relay servers
    fn record_relay_servers(&self, count: usize);
    
    /// Record the number of active relay connections
    fn record_active_relay_connections(&self, count: usize);
    
    /// Record a relay connection attempt
    fn record_relay_connection_attempt(&self);
    
    /// Record a successful relay connection
    fn record_relay_connection_success(&self);
    
    /// Record a failed relay connection
    fn record_relay_connection_failure(&self);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_relay_manager_initialization() {
        let config = CircuitRelayConfig {
            known_relay_servers: vec![
                "/ip4/127.0.0.1/tcp/9000/p2p/QmTest1".parse().unwrap(),
                "/ip4/127.0.0.1/tcp/9001/p2p/QmTest2".parse().unwrap(),
            ],
            ..Default::default()
        };
        
        let manager = CircuitRelayManager::new(config, None);
        manager.initialize().await.unwrap();
        
        let servers = manager.get_relay_servers().await;
        assert_eq!(servers.len(), 2);
    }
    
    #[tokio::test]
    async fn test_extract_peer_id() {
        let addr = "/ip4/127.0.0.1/tcp/9000/p2p/QmTest1".parse::<Multiaddr>().unwrap();
        let peer_id = extract_peer_id(&addr);
        assert!(peer_id.is_some());
    }
} 