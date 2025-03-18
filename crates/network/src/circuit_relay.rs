//! Circuit relay for ICN Network
//!
//! This module implements a circuit relay protocol that enables nodes behind NATs 
//! to connect to other nodes through publicly accessible relay nodes.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Debug;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

use async_trait::async_trait;
use futures::StreamExt;
use libp2p::{
    self,
    core::{muxing::StreamMuxerBox, transport::OrTransport, upgrade},
    gossipsub::{self, IdentTopic, MessageAuthenticity, MessageId, ValidationMode},
    identify, kad, mdns, noise, ping, relay,
    swarm::{self, ConnectionError, NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Transport,
};
use libp2p::swarm::behaviour::toggle::Toggle;
use multiaddr::Protocol;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};
use rand::seq::SliceRandom;

use crate::{NetworkError, NetworkResult, metrics::NetworkMetrics};
use crate::reputation::{ReputationManager, ReputationChange};

/// Configuration for circuit relay
#[derive(Debug, Clone)]
pub struct CircuitRelayConfig {
    /// Maximum number of relay connections per peer
    pub max_connections: usize,
    /// Maximum number of circuits per relay
    pub max_circuits: usize,
    /// Circuit timeout duration
    pub circuit_timeout: Duration,
    /// Connection pool size per relay
    pub pool_size: usize,
    /// Pool connection timeout
    pub pool_timeout: Duration,
    /// Minimum number of available relays
    pub min_relays: usize,
    /// Maximum latency for relay selection (ms)
    pub max_relay_latency: u64,
    /// Enable automatic relay failover
    pub enable_failover: bool,
    /// Failover timeout duration
    pub failover_timeout: Duration,
    /// Maximum retry attempts for failover
    pub max_retry_attempts: u32,
}

impl Default for CircuitRelayConfig {
    fn default() -> Self {
        Self {
            max_connections: 50,
            max_circuits: 20,
            circuit_timeout: Duration::from_secs(60),
            pool_size: 10,
            pool_timeout: Duration::from_secs(30),
            min_relays: 3,
            max_relay_latency: 200,
            enable_failover: true,
            failover_timeout: Duration::from_secs(10),
            max_retry_attempts: 3,
        }
    }
}

/// Status of a relay connection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelayStatus {
    /// Connection is available
    Available,
    /// Connection is in use
    InUse,
    /// Connection is being established
    Connecting,
    /// Connection failed
    Failed,
}

/// A pooled relay connection
#[derive(Debug)]
struct PooledConnection {
    /// Relay peer ID
    relay_id: PeerId,
    /// Connection status
    status: RelayStatus,
    /// When the connection was created
    created_at: Instant,
    /// Last time the connection was used
    last_used: Instant,
    /// Number of circuits using this connection
    circuit_count: usize,
    /// Average latency in milliseconds
    avg_latency: u64,
    /// Number of failed operations
    failure_count: u32,
}

/// A relay server with its connection pool
#[derive(Debug)]
struct RelayServer {
    /// Relay peer ID
    peer_id: PeerId,
    /// Relay addresses
    addresses: Vec<Multiaddr>,
    /// Connection pool
    pool: VecDeque<PooledConnection>,
    /// Total number of active circuits
    active_circuits: usize,
    /// Last health check time
    last_health_check: Instant,
    /// Success rate (0.0 - 1.0)
    success_rate: f32,
    /// Average latency in milliseconds
    avg_latency: u64,
    /// Whether this relay is currently available
    is_available: bool,
}

/// Circuit relay manager
pub struct CircuitRelayManager {
    /// Configuration
    config: CircuitRelayConfig,
    /// Available relay servers
    relays: Arc<RwLock<HashMap<PeerId, RelayServer>>>,
    /// Active circuits
    circuits: Arc<RwLock<HashMap<(PeerId, PeerId), Instant>>>,
    /// Failed relays with their failure time
    failed_relays: Arc<RwLock<HashMap<PeerId, Instant>>>,
    /// Reputation manager
    reputation: Arc<ReputationManager>,
    /// Metrics collection
    metrics: Option<Arc<NetworkMetrics>>,
}

// Add this impl to ensure CircuitRelayManager is Send + Sync
unsafe impl Send for CircuitRelayManager {}
unsafe impl Sync for CircuitRelayManager {}

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
    pub fn new(
        config: CircuitRelayConfig,
        reputation: Arc<ReputationManager>,
        metrics: Option<Arc<NetworkMetrics>>,
    ) -> Self {
        Self {
            config,
            relays: Arc::new(RwLock::new(HashMap::new())),
            circuits: Arc::new(RwLock::new(HashMap::new())),
            failed_relays: Arc::new(RwLock::new(HashMap::new())),
            reputation,
            metrics,
        }
    }
    
    /// Add a relay server
    pub async fn add_relay(
        &self,
        peer_id: PeerId,
        addresses: Vec<Multiaddr>,
    ) -> NetworkResult<()> {
        let mut relays = self.relays.write().await;
        
        // Create new relay server with empty pool
        let relay = RelayServer {
            peer_id,
            addresses,
            pool: VecDeque::with_capacity(self.config.pool_size),
            active_circuits: 0,
            last_health_check: Instant::now(),
            success_rate: 1.0,
            avg_latency: 0,
            is_available: true,
        };
        
        relays.insert(peer_id, relay);
        
        // Initialize connection pool
        self.initialize_pool(peer_id).await?;
        
        Ok(())
    }
    
    /// Initialize connection pool for a relay
    async fn initialize_pool(&self, relay_id: PeerId) -> NetworkResult<()> {
        let mut relays = self.relays.write().await;
        
        if let Some(relay) = relays.get_mut(&relay_id) {
            // Create initial pool connections
            for _ in 0..self.config.pool_size {
                let conn = PooledConnection {
                    relay_id,
                    status: RelayStatus::Available,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    circuit_count: 0,
                    avg_latency: 0,
                    failure_count: 0,
                };
                
                relay.pool.push_back(conn);
            }
        }
        
        Ok(())
    }
    
    /// Get the best available relay for a circuit
    pub async fn select_relay(&self, target: PeerId) -> NetworkResult<PeerId> {
        let relays = self.relays.read().await;
        let failed = self.failed_relays.read().await;
        
        // Filter available relays
        let available: Vec<_> = relays.values()
            .filter(|relay| {
                relay.is_available &&
                relay.active_circuits < self.config.max_circuits &&
                relay.avg_latency <= self.config.max_relay_latency &&
                !failed.contains_key(&relay.peer_id)
            })
            .collect();
            
        if available.is_empty() {
            return Err(NetworkError::NoRelaysAvailable);
        }
        
        // Sort by score (combination of latency and success rate)
        let mut scored: Vec<_> = available.iter()
            .map(|relay| {
                let latency_score = 1.0 - (relay.avg_latency as f32 / self.config.max_relay_latency as f32);
                let score = relay.success_rate * latency_score;
                (relay, score)
            })
            .collect();
            
        scored.sort_by(|(_, a), (_, b)| b.partial_cmp(a).unwrap());
        
        // Select randomly from top 3 relays
        let top_relays = scored.iter().take(3).collect::<Vec<_>>();
        if let Some((relay, _)) = top_relays.choose(&mut rand::thread_rng()) {
            Ok(relay.peer_id)
        } else {
            Err(NetworkError::NoRelaysAvailable)
        }
    }
    
    /// Get an available connection from a relay's pool
    pub async fn get_connection(&self, relay_id: PeerId) -> NetworkResult<PooledConnection> {
        let mut relays = self.relays.write().await;
        
        if let Some(relay) = relays.get_mut(&relay_id) {
            // Try to get an available connection
            for _ in 0..relay.pool.len() {
                if let Some(mut conn) = relay.pool.pop_front() {
                    if conn.status == RelayStatus::Available {
                        // Update connection state
                        conn.status = RelayStatus::InUse;
                        conn.last_used = Instant::now();
                        conn.circuit_count += 1;
                        
                        // Put connection back at end of queue
                        relay.pool.push_back(conn.clone());
                        return Ok(conn);
                    } else {
                        // Put unavailable connection back
                        relay.pool.push_back(conn);
                    }
                }
            }
            
            // No available connections, create new one if possible
            if relay.pool.len() < self.config.pool_size {
                let conn = PooledConnection {
                    relay_id,
                    status: RelayStatus::InUse,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    circuit_count: 1,
                    avg_latency: relay.avg_latency,
                    failure_count: 0,
                };
                
                relay.pool.push_back(conn.clone());
                return Ok(conn);
            }
        }
        
        Err(NetworkError::NoConnectionsAvailable)
    }
    
    /// Release a connection back to the pool
    pub async fn release_connection(
        &self,
        relay_id: PeerId,
        mut conn: PooledConnection,
    ) -> NetworkResult<()> {
        let mut relays = self.relays.write().await;
        
        if let Some(relay) = relays.get_mut(&relay_id) {
            // Update connection state
            conn.status = RelayStatus::Available;
            conn.circuit_count -= 1;
            
            // Update relay metrics
            relay.active_circuits -= 1;
            
            // Put connection back in pool
            relay.pool.push_back(conn);
        }
        
        Ok(())
    }
    
    /// Handle relay failure and initiate failover if enabled
    pub async fn handle_relay_failure(
        &self,
        relay_id: PeerId,
        error: NetworkError,
    ) -> NetworkResult<Option<PeerId>> {
        let mut relays = self.relays.write().await;
        let mut failed = self.failed_relays.write().await;
        
        // Mark relay as failed
        if let Some(relay) = relays.get_mut(&relay_id) {
            relay.is_available = false;
            relay.success_rate *= 0.9; // Decay success rate
            failed.insert(relay_id, Instant::now());
            
            // Update reputation
            self.reputation.record_change(relay_id, ReputationChange::RelayFailure).await?;
        }
        
        // Attempt failover if enabled
        if self.config.enable_failover {
            for _ in 0..self.config.max_retry_attempts {
                if let Ok(new_relay) = self.select_relay(relay_id).await {
                    return Ok(Some(new_relay));
                }
                tokio::time::sleep(self.config.failover_timeout).await;
            }
        }
        
        Ok(None)
    }
    
    /// Perform health check on relay servers
    pub async fn health_check(&self) -> NetworkResult<()> {
        let mut relays = self.relays.write().await;
        let mut failed = self.failed_relays.write().await;
        
        // Check each relay
        for relay in relays.values_mut() {
            if Instant::now().duration_since(relay.last_health_check) >= Duration::from_secs(60) {
                // Perform health check (ping, measure latency, etc)
                let (is_healthy, latency) = self.check_relay_health(relay.peer_id).await?;
                
                relay.is_available = is_healthy;
                if is_healthy {
                    // Update metrics
                    relay.avg_latency = (relay.avg_latency + latency) / 2;
                    relay.success_rate = 0.9 * relay.success_rate + 0.1; // Slowly recover
                    failed.remove(&relay.peer_id);
                    
                    // Update reputation
                    self.reputation.record_change(relay.peer_id, ReputationChange::RelaySuccess).await?;
                } else {
                    failed.insert(relay.peer_id, Instant::now());
                }
                
                relay.last_health_check = Instant::now();
            }
        }
        
        Ok(())
    }
    
    /// Check health of a specific relay
    async fn check_relay_health(&self, relay_id: PeerId) -> NetworkResult<(bool, u64)> {
        // TODO: Implement actual health check
        // For now, return dummy values
        Ok((true, 50))
    }
    
    /// Clean up expired circuits and connections
    pub async fn cleanup(&self) -> NetworkResult<()> {
        let mut relays = self.relays.write().await;
        let mut circuits = self.circuits.write().await;
        let mut failed = self.failed_relays.write().await;
        
        // Clean up expired circuits
        circuits.retain(|_, created_at| {
            created_at.elapsed() < self.config.circuit_timeout
        });
        
        // Clean up failed relays
        failed.retain(|_, failed_at| {
            failed_at.elapsed() < Duration::from_secs(300) // Remove after 5 minutes
        });
        
        // Clean up expired connections and rebalance pools
        for relay in relays.values_mut() {
            // Remove expired connections
            relay.pool.retain(|conn| {
                conn.created_at.elapsed() < Duration::from_secs(3600) && // Max 1 hour old
                conn.failure_count < 5 // Max 5 failures
            });
            
            // Add new connections if pool is depleted
            while relay.pool.len() < self.config.pool_size {
                let conn = PooledConnection {
                    relay_id: relay.peer_id,
                    status: RelayStatus::Available,
                    created_at: Instant::now(),
                    last_used: Instant::now(),
                    circuit_count: 0,
                    avg_latency: relay.avg_latency,
                    failure_count: 0,
                };
                
                relay.pool.push_back(conn);
            }
        }
        
        Ok(())
    }

    /// Establish a circuit to a target peer through a relay
    pub async fn establish_circuit(
        &self,
        target: PeerId,
        relay_hint: Option<PeerId>,
    ) -> NetworkResult<RelayConnectionInfo> {
        // Try specified relay first if provided
        if let Some(relay_id) = relay_hint {
            if let Ok(conn) = self.try_establish_circuit(target, relay_id).await {
                return Ok(conn);
            }
        }

        // Otherwise select best available relay
        let relay_id = self.select_relay(target).await?;
        self.try_establish_circuit(target, relay_id).await
    }

    /// Try to establish a circuit through a specific relay
    async fn try_establish_circuit(
        &self,
        target: PeerId,
        relay_id: PeerId,
    ) -> NetworkResult<RelayConnectionInfo> {
        // Get a connection from the relay's pool
        let conn = self.get_connection(relay_id).await?;
        
        // Track circuit establishment start time
        let start_time = Instant::now();
        
        // Create circuit key
        let circuit_key = (target, relay_id);
        
        // Record circuit attempt in metrics
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_connection_attempt();
        }
        
        // Add to active circuits
        let mut circuits = self.circuits.write().await;
        circuits.insert(circuit_key, Instant::now());
        
        // Create connection info
        let info = RelayConnectionInfo {
            dest_peer_id: target,
            relay_peer_id: relay_id,
            established_at: Instant::now(),
            ttl: self.config.circuit_timeout,
            reservation_id: None, // TODO: Implement relay reservations
        };
        
        // Update relay metrics
        let mut relays = self.relays.write().await;
        if let Some(relay) = relays.get_mut(&relay_id) {
            relay.active_circuits += 1;
            relay.avg_latency = (relay.avg_latency + start_time.elapsed().as_millis() as u64) / 2;
            relay.success_rate = 0.9 * relay.success_rate + 0.1;
        }
        
        // Update reputation
        self.reputation.record_change(relay_id, ReputationChange::RelaySuccess).await?;
        
        // Record success in metrics
        if let Some(metrics) = &self.metrics {
            metrics.record_relay_connection_success();
        }
        
        Ok(info)
    }

    /// Close a circuit
    pub async fn close_circuit(
        &self,
        target: PeerId,
        relay_id: PeerId,
    ) -> NetworkResult<()> {
        // Remove from active circuits
        let mut circuits = self.circuits.write().await;
        circuits.remove(&(target, relay_id));
        
        // Get relay connection
        if let Ok(conn) = self.get_connection(relay_id).await {
            // Release connection back to pool
            self.release_connection(relay_id, conn).await?;
        }
        
        // Update relay metrics
        let mut relays = self.relays.write().await;
        if let Some(relay) = relays.get_mut(&relay_id) {
            relay.active_circuits = relay.active_circuits.saturating_sub(1);
        }
        
        Ok(())
    }

    /// Get information about active circuits
    pub async fn get_active_circuits(&self) -> Vec<RelayConnectionInfo> {
        let circuits = self.circuits.read().await;
        let mut active = Vec::new();
        
        for ((dest, relay), established) in circuits.iter() {
            active.push(RelayConnectionInfo {
                dest_peer_id: *dest,
                relay_peer_id: *relay,
                established_at: *established,
                ttl: self.config.circuit_timeout,
                reservation_id: None,
            });
        }
        
        active
    }

    /// Get statistics about relay usage
    pub async fn get_relay_stats(&self) -> HashMap<PeerId, RelayServerInfo> {
        let relays = self.relays.read().await;
        let mut stats = HashMap::new();
        
        for relay in relays.values() {
            let successful = relay.pool.iter()
                .filter(|conn| conn.failure_count == 0)
                .count();
                
            let failed = relay.pool.iter()
                .map(|conn| conn.failure_count as usize)
                .sum();
                
            stats.insert(relay.peer_id, RelayServerInfo {
                peer_id: relay.peer_id,
                addresses: relay.addresses.clone(),
                last_used: relay.pool.iter()
                    .map(|conn| conn.last_used)
                    .max()
                    .unwrap_or_else(Instant::now),
                successful_connections: successful,
                failed_connections: failed,
            });
        }
        
        stats
    }

    /// Start periodic maintenance tasks
    pub async fn start_maintenance(&self) {
        let manager = self.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                
                // Perform health checks
                if let Err(e) = manager.health_check().await {
                    error!("Relay health check failed: {}", e);
                }
                
                // Clean up expired circuits and connections
                if let Err(e) = manager.cleanup().await {
                    error!("Relay cleanup failed: {}", e);
                }
                
                // Update metrics
                if let Some(metrics) = &manager.metrics {
                    let relays = manager.relays.read().await;
                    metrics.record_relay_servers(relays.len());
                    
                    let circuits = manager.circuits.read().await;
                    metrics.record_active_relay_connections(circuits.len());
                }
            }
        });
    }

    /// Initialize the relay manager
    pub async fn initialize(&self) -> crate::NetworkResult<()> {
        // Initialize relay connections if any are configured
        let relays = self.relays.read().await;
        
        for peer_id in relays.keys() {
            if let Err(e) = self.initialize_pool(*peer_id).await {
                tracing::warn!("Failed to initialize relay pool for {}: {}", peer_id, e);
            }
        }
        
        Ok(())
    }

    /// Start a background task to clean up relay connections periodically
    pub fn start_cleanup_task(&self) {
        let manager = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            
            loop {
                interval.tick().await;
                if let Err(e) = manager.cleanup().await {
                    tracing::error!("Error in relay cleanup task: {}", e);
                }
            }
        });
    }

    /// Connect to a peer via a relay server
    pub async fn connect_via_relay(&self, peer_id: PeerId) -> crate::NetworkResult<Multiaddr> {
        // Find the best relay for this connection
        let relay_id = self.select_relay(peer_id).await?;
        
        // Get relay's address
        let relay_addr = {
            let relays = self.relays.read().await;
            let relay = relays.get(&relay_id)
                .ok_or_else(|| crate::NetworkError::RelayConnectionError(
                    format!("Relay {} not found", relay_id)
                ))?;
                
            if relay.addresses.is_empty() {
                return Err(crate::NetworkError::RelayConnectionError(
                    format!("No addresses for relay {}", relay_id)
                ));
            }
            
            // Use the first address
            relay.addresses[0].clone()
        };
        
        // Create a relay address for the destination peer
        let mut relayed_addr = relay_addr.clone();
        relayed_addr.push(Protocol::P2p(peer_id.into()));
        
        Ok(relayed_addr)
    }

    /// Check if a connection is relayed
    pub async fn is_relayed_connection(&self, peer_id: PeerId) -> bool {
        let circuits = self.circuits.read().await;
        
        for (relay_pair, _) in circuits.iter() {
            if relay_pair.0 == peer_id {
                return true;
            }
        }
        
        false
    }

    /// Get the relay used for a specific connection
    pub async fn get_relay_for_connection(&self, peer_id: PeerId) -> Option<PeerId> {
        let circuits = self.circuits.read().await;
        
        for ((dest, relay), _) in circuits.iter() {
            if *dest == peer_id {
                return Some(*relay);
            }
        }
        
        None
    }

    /// Get a list of all relay servers
    pub async fn get_relay_servers(&self) -> Vec<String> {
        let relays = self.relays.read().await;
        relays.keys().map(|id| id.to_string()).collect()
    }

    /// Add a relay server
    pub async fn add_relay_server(&self, peer_id: PeerId, addresses: Vec<Multiaddr>) -> crate::NetworkResult<()> {
        self.add_relay(peer_id, addresses).await
    }
}

impl Clone for CircuitRelayManager {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            relays: self.relays.clone(),
            circuits: self.circuits.clone(),
            failed_relays: self.failed_relays.clone(),
            reputation: self.reputation.clone(),
            metrics: self.metrics.clone(),
        }
    }
}

impl Clone for PooledConnection {
    fn clone(&self) -> Self {
        Self {
            relay_id: self.relay_id,
            status: self.status,
            created_at: self.created_at,
            last_used: self.last_used,
            circuit_count: self.circuit_count,
            avg_latency: self.avg_latency,
            failure_count: self.failure_count,
        }
    }
}

/// Create a relay transport
pub fn create_relay_transport<T>(
    transport: T,
    relay_config: &CircuitRelayConfig,
) -> NetworkResult<T> 
where
    T: Transport + Clone + Send + 'static,
    T::Output: Send + 'static,
    T::Error: std::error::Error + Send + Sync + 'static,
{
    // In libp2p 0.55, the relay transport API has changed
    // We'll just return the original transport since relay functionality
    // will need to be reimplemented with the current libp2p version
    
    // Below is placeholder code just to get it to compile
    debug!("Creating relay transport with config: {:?}", relay_config);
    Ok(transport)
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

/// Circuit relay network behavior
pub struct CircuitRelayBehaviour {
    // Circuit Relay implementation fields
    // ... etc
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
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_relay_manager() {
        // Create test configuration
        let config = CircuitRelayConfig::default();
        let reputation = Arc::new(ReputationManager::new(Default::default()));
        let manager = CircuitRelayManager::new(config, reputation, None);
        
        // Add some test relays
        let relay1 = PeerId::random();
        let relay2 = PeerId::random();
        
        manager.add_relay(relay1, vec!["/ip4/127.0.0.1/tcp/10000".parse().unwrap()]).await.unwrap();
        manager.add_relay(relay2, vec!["/ip4/127.0.0.1/tcp/10001".parse().unwrap()]).await.unwrap();
        
        // Test relay selection
        let target = PeerId::random();
        let selected = manager.select_relay(target).await.unwrap();
        assert!(selected == relay1 || selected == relay2);
        
        // Test circuit establishment
        let circuit = manager.establish_circuit(target, None).await.unwrap();
        assert_eq!(circuit.dest_peer_id, target);
        assert!(circuit.relay_peer_id == relay1 || circuit.relay_peer_id == relay2);
        
        // Test active circuits
        let active = manager.get_active_circuits().await;
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].dest_peer_id, target);
        
        // Test circuit closure
        manager.close_circuit(target, circuit.relay_peer_id).await.unwrap();
        let active = manager.get_active_circuits().await;
        assert_eq!(active.len(), 0);
    }
    
    #[tokio::test]
    async fn test_relay_failover() {
        // Create test configuration
        let config = CircuitRelayConfig::default();
        let reputation = Arc::new(ReputationManager::new(Default::default()));
        let manager = CircuitRelayManager::new(config, reputation, None);
        
        // Add test relays
        let relay1 = PeerId::random();
        let relay2 = PeerId::random();
        let relay3 = PeerId::random();
        
        manager.add_relay(relay1, vec!["/ip4/127.0.0.1/tcp/10000".parse().unwrap()]).await.unwrap();
        manager.add_relay(relay2, vec!["/ip4/127.0.0.1/tcp/10001".parse().unwrap()]).await.unwrap();
        manager.add_relay(relay3, vec!["/ip4/127.0.0.1/tcp/10002".parse().unwrap()]).await.unwrap();
        
        // Simulate relay failure
        let target = PeerId::random();
        manager.handle_relay_failure(relay1, NetworkError::ConnectionFailed).await.unwrap();
        
        // Test failover
        let circuit = manager.establish_circuit(target, None).await.unwrap();
        assert!(circuit.relay_peer_id == relay2 || circuit.relay_peer_id == relay3);
        assert_ne!(circuit.relay_peer_id, relay1);
    }
    
    #[tokio::test]
    async fn test_connection_pool() {
        // Create test configuration
        let config = CircuitRelayConfig::default();
        let reputation = Arc::new(ReputationManager::new(Default::default()));
        let manager = CircuitRelayManager::new(config, reputation, None);
        
        // Add test relay
        let relay = PeerId::random();
        manager.add_relay(relay, vec!["/ip4/127.0.0.1/tcp/10000".parse().unwrap()]).await.unwrap();
        
        // Get connections from pool
        let conn1 = manager.get_connection(relay).await.unwrap();
        let conn2 = manager.get_connection(relay).await.unwrap();
        
        // Pool should be at capacity
        assert!(manager.get_connection(relay).await.is_err());
        
        // Release connection
        manager.release_connection(relay, conn1).await.unwrap();
        
        // Should be able to get another connection
        let conn3 = manager.get_connection(relay).await.unwrap();
        assert_eq!(conn3.relay_id, relay);
    }
    
    #[tokio::test]
    async fn test_relay_health_check() {
        // Create test configuration
        let config = CircuitRelayConfig::default();
        let reputation = Arc::new(ReputationManager::new(Default::default()));
        let manager = CircuitRelayManager::new(config, reputation, None);
        
        // Add test relay
        let relay = PeerId::random();
        manager.add_relay(relay, vec!["/ip4/127.0.0.1/tcp/10000".parse().unwrap()]).await.unwrap();
        
        // Perform health check
        manager.health_check().await.unwrap();
        
        // Get relay stats
        let stats = manager.get_relay_stats().await;
        assert!(stats.contains_key(&relay));
        
        let relay_info = stats.get(&relay).unwrap();
        assert_eq!(relay_info.successful_connections, 0);
        assert_eq!(relay_info.failed_connections, 0);
    }
} 