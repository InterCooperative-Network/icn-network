//! Tunneling for the ICN overlay network
//! 
//! This module provides tunneling functionality to encapsulate IPv6 traffic
//! between ICN nodes in the overlay network. It supports various tunnel types
//! such as direct IPv6, WireGuard, TLS, and onion-routed tunnels.

use std::{
    collections::HashMap,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{Arc, RwLock},
    time::Duration,
};
use serde::{Serialize, Deserialize};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::{mpsc, Mutex},
    time::timeout,
};
use tracing::{debug, error, info, trace, warn};
use thiserror::Error;

use super::{OverlayAddress, TunnelType, TunnelInfo};
use crate::error::{Result, NetworkError};

/// Tunnel-related errors
#[derive(Error, Debug)]
pub enum TunnelError {
    #[error("Tunnel creation failed: {0}")]
    CreationFailed(String),
    #[error("Tunnel connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Tunnel not found: {0}")]
    NotFound(String),
    #[error("Tunnel already exists: {0}")]
    AlreadyExists(String),
    #[error("Incompatible tunnel type: {0}")]
    IncompatibleType(String),
    #[error("Tunnel closed")]
    Closed,
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Status of a tunnel
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TunnelStatus {
    /// Tunnel is initializing
    Initializing,
    /// Tunnel is connecting
    Connecting,
    /// Tunnel is connected and active
    Connected,
    /// Tunnel is disconnected but can be reconnected
    Disconnected,
    /// Tunnel has failed
    Failed,
    /// Tunnel is closed permanently
    Closed,
}

/// WireGuard tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardConfig {
    /// WireGuard private key (base64)
    pub private_key: String,
    /// WireGuard public key (base64)
    pub public_key: String,
    /// WireGuard peer public key (base64)
    pub peer_public_key: String,
    /// Pre-shared key (base64, optional)
    pub preshared_key: Option<String>,
    /// WireGuard listen port
    pub listen_port: u16,
    /// WireGuard peer endpoint
    pub peer_endpoint: SocketAddr,
    /// Allowed IPs (typically the overlay subnet)
    pub allowed_ips: Vec<String>,
    /// Keepalive interval in seconds
    pub persistent_keepalive: u16,
}

/// Statistics for a tunnel
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TunnelStats {
    /// Number of bytes received
    pub bytes_rx: u64,
    /// Number of bytes transmitted
    pub bytes_tx: u64,
    /// Number of packets received
    pub packets_rx: u64,
    /// Number of packets transmitted
    pub packets_tx: u64,
    /// Number of errors
    pub errors: u64,
    /// Last activity timestamp
    pub last_activity: i64,
    /// Creation timestamp
    pub created_at: i64,
    /// Connection established timestamp
    pub connected_at: Option<i64>,
}

/// Manager for tunnels in the overlay network
pub struct TunnelManager {
    /// Active tunnels
    tunnels: Arc<RwLock<HashMap<String, TunnelInfo>>>,
    /// Tunnel statistics
    stats: Arc<RwLock<HashMap<String, TunnelStats>>>,
    /// WireGuard configurations
    wireguard_configs: Arc<RwLock<HashMap<String, WireGuardConfig>>>,
    /// Default tunnel type
    default_tunnel_type: TunnelType,
    /// Channel for sending data through tunnels
    data_tx: mpsc::Sender<(String, Vec<u8>)>,
    /// Channel for receiving data from tunnels
    data_rx: Arc<Mutex<mpsc::Receiver<(String, Vec<u8>)>>>,
}

impl TunnelManager {
    /// Create a new tunnel manager
    pub fn new() -> Self {
        let (data_tx, data_rx) = mpsc::channel(1000);
        
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(HashMap::new())),
            wireguard_configs: Arc::new(RwLock::new(HashMap::new())),
            default_tunnel_type: TunnelType::Direct,
            data_tx,
            data_rx: Arc::new(Mutex::new(data_rx)),
        }
    }
    
    /// Start the tunnel manager
    pub async fn start(&self) -> Result<()> {
        // Start background tasks for monitoring tunnels
        self.start_tunnel_monitoring().await?;
        
        Ok(())
    }
    
    /// Create a new tunnel
    pub async fn create_tunnel(
        &self, 
        local_addr: &OverlayAddress, 
        remote_addr: &OverlayAddress, 
        tunnel_type: TunnelType
    ) -> Result<TunnelInfo> {
        // Check if a tunnel to this destination already exists
        {
            let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
            for tunnel in tunnels.values() {
                if tunnel.remote_overlay_addr == *remote_addr && 
                   tunnel.local_overlay_addr == *local_addr &&
                   tunnel.tunnel_type == tunnel_type &&
                   tunnel.active {
                    // Tunnel already exists and is active
                    return Ok(tunnel.clone());
                }
            }
        }
        
        // Generate a tunnel ID
        let tunnel_id = format!("tunnel-{}-{}-{}", 
            local_addr.to_ipv6().segments()[7],
            remote_addr.to_ipv6().segments()[7],
            chrono::Utc::now().timestamp() % 1000
        );
        
        // Create remote endpoint
        let remote_endpoint = match tunnel_type {
            TunnelType::Direct => {
                // For direct, use the IPv6 address directly
                SocketAddr::new(
                    IpAddr::V6(remote_addr.to_ipv6()),
                    4789 // Default VXLAN port
                )
            },
            TunnelType::WireGuard => {
                // For WireGuard, we'd use a pre-configured endpoint or discovery
                SocketAddr::new(
                    IpAddr::V6(remote_addr.to_ipv6()),
                    51820 // Default WireGuard port
                )
            },
            TunnelType::Tls => {
                // For TLS, use a TLS port
                SocketAddr::new(
                    IpAddr::V6(remote_addr.to_ipv6()),
                    8443 // TLS port
                )
            },
            TunnelType::Onion => {
                // For onion routing, endpoint depends on the onion circuit
                SocketAddr::new(
                    IpAddr::V6(remote_addr.to_ipv6()),
                    9050 // Default Tor port
                )
            },
        };
        
        // Create tunnel info
        let tunnel_info = TunnelInfo {
            id: tunnel_id.clone(),
            tunnel_type,
            remote_endpoint,
            local_overlay_addr: local_addr.clone(),
            remote_overlay_addr: remote_addr.clone(),
            mtu: 1420, // Default MTU
            active: true,
            last_activity: chrono::Utc::now().timestamp(),
        };
        
        // Create tunnel statistics
        let stats = TunnelStats {
            bytes_rx: 0,
            bytes_tx: 0,
            packets_rx: 0,
            packets_tx: 0,
            errors: 0,
            last_activity: chrono::Utc::now().timestamp(),
            created_at: chrono::Utc::now().timestamp(),
            connected_at: None,
        };
        
        // Set up the actual tunnel
        match tunnel_type {
            TunnelType::Direct => {
                // Direct IPv6 doesn't need special setup
                info!("Created direct IPv6 tunnel to {}", remote_addr);
            },
            TunnelType::WireGuard => {
                // Set up WireGuard tunnel
                self.setup_wireguard_tunnel(&tunnel_id, local_addr, remote_addr).await?;
            },
            TunnelType::Tls => {
                // Set up TLS tunnel
                self.setup_tls_tunnel(&tunnel_id, local_addr, remote_addr).await?;
            },
            TunnelType::Onion => {
                // Set up onion tunnel
                self.setup_onion_tunnel(&tunnel_id, local_addr, remote_addr).await?;
            },
        }
        
        // Store tunnel info and stats
        {
            let mut tunnels = self.tunnels.write().map_err(|_| NetworkError::LockError)?;
            tunnels.insert(tunnel_id.clone(), tunnel_info.clone());
            
            let mut stats_map = self.stats.write().map_err(|_| NetworkError::LockError)?;
            stats_map.insert(tunnel_id.clone(), stats);
        }
        
        // Start tunnel monitoring task
        self.start_tunnel_handler(&tunnel_id).await?;
        
        Ok(tunnel_info)
    }
    
    /// Set up a WireGuard tunnel
    async fn setup_wireguard_tunnel(
        &self, 
        tunnel_id: &str,
        local_addr: &OverlayAddress,
        remote_addr: &OverlayAddress
    ) -> Result<()> {
        // In a real implementation, this would generate WireGuard keys
        // and set up the WireGuard interface
        
        // Generate WireGuard keys (simulated)
        let private_key = "4GgcpbkYMv9L8XCh8vGHdk4Hs9Rx9jzLTNGJPZzTGVc=".to_string();
        let public_key = "Ak59nJ3iKYfXHQONJLJpS3CFOP8n9SvKR4MlrPF+txo=".to_string();
        let peer_public_key = "YLPvfXsza4BLiT3EqnNpOdhS5WZleY5FILVdlXCkHjE=".to_string();
        
        // Create WireGuard config
        let wg_config = WireGuardConfig {
            private_key,
            public_key,
            peer_public_key,
            preshared_key: None,
            listen_port: 51820,
            peer_endpoint: SocketAddr::new(
                IpAddr::V6(remote_addr.to_ipv6()),
                51820
            ),
            allowed_ips: vec![format!("{}/128", remote_addr.to_ipv6())],
            persistent_keepalive: 25,
        };
        
        // Store WireGuard config
        let mut wg_configs = self.wireguard_configs.write().map_err(|_| NetworkError::LockError)?;
        wg_configs.insert(tunnel_id.to_string(), wg_config);
        
        info!("Set up WireGuard tunnel to {}", remote_addr);
        
        Ok(())
    }
    
    /// Set up a TLS tunnel
    async fn setup_tls_tunnel(
        &self, 
        tunnel_id: &str,
        local_addr: &OverlayAddress,
        remote_addr: &OverlayAddress
    ) -> Result<()> {
        // In a real implementation, this would set up TLS certificates
        // and establish a TLS connection
        
        info!("Set up TLS tunnel to {}", remote_addr);
        
        Ok(())
    }
    
    /// Set up an onion tunnel
    async fn setup_onion_tunnel(
        &self, 
        tunnel_id: &str,
        local_addr: &OverlayAddress,
        remote_addr: &OverlayAddress
    ) -> Result<()> {
        // In a real implementation, this would set up an onion circuit
        
        info!("Set up onion tunnel to {}", remote_addr);
        
        Ok(())
    }
    
    /// Send data through a tunnel
    pub async fn send_through_tunnel(&self, tunnel_id: &str, data: &[u8]) -> Result<()> {
        // Check if tunnel exists and is active
        let tunnel = {
            let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
            tunnels.get(tunnel_id).cloned()
        };
        
        if let Some(tunnel) = tunnel {
            if !tunnel.active {
                return Err(NetworkError::Other(format!("Tunnel {} is not active", tunnel_id)));
            }
            
            // Update statistics
            {
                let mut stats_map = self.stats.write().map_err(|_| NetworkError::LockError)?;
                if let Some(stats) = stats_map.get_mut(tunnel_id) {
                    stats.bytes_tx += data.len() as u64;
                    stats.packets_tx += 1;
                    stats.last_activity = chrono::Utc::now().timestamp();
                }
            }
            
            // Send data through the channel
            self.data_tx.send((tunnel_id.to_string(), data.to_vec())).await
                .map_err(|_| NetworkError::Other("Channel closed".into()))?;
                
            trace!("Sent {} bytes through tunnel {}", data.len(), tunnel_id);
            
            Ok(())
        } else {
            Err(NetworkError::Other(format!("Tunnel {} not found", tunnel_id)))
        }
    }
    
    /// Receive data from a tunnel (with timeout)
    pub async fn receive_from_tunnel(&self, timeout_ms: u64) -> Result<(String, Vec<u8>)> {
        let mut rx = self.data_rx.lock().await;
        
        match timeout(Duration::from_millis(timeout_ms), rx.recv()).await {
            Ok(Some((tunnel_id, data))) => {
                // Update statistics
                {
                    let mut stats_map = self.stats.write().map_err(|_| NetworkError::LockError)?;
                    if let Some(stats) = stats_map.get_mut(&tunnel_id) {
                        stats.bytes_rx += data.len() as u64;
                        stats.packets_rx += 1;
                        stats.last_activity = chrono::Utc::now().timestamp();
                    }
                }
                
                trace!("Received {} bytes from tunnel {}", data.len(), tunnel_id);
                
                Ok((tunnel_id, data))
            },
            Ok(None) => Err(NetworkError::Other("Channel closed".into())),
            Err(_) => Err(NetworkError::Other("Timeout".into())),
        }
    }
    
    /// Close a tunnel
    pub async fn close_tunnel(&self, tunnel_id: &str) -> Result<()> {
        // Check if tunnel exists
        let tunnel = {
            let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
            tunnels.get(tunnel_id).cloned()
        };
        
        if let Some(mut tunnel) = tunnel {
            // Update tunnel status
            tunnel.active = false;
            
            // Store updated tunnel info
            {
                let mut tunnels = self.tunnels.write().map_err(|_| NetworkError::LockError)?;
                tunnels.insert(tunnel_id.to_string(), tunnel.clone());
            }
            
            // Clean up the tunnel based on type
            match tunnel.tunnel_type {
                TunnelType::WireGuard => {
                    // Clean up WireGuard tunnel
                    let mut wg_configs = self.wireguard_configs.write().map_err(|_| NetworkError::LockError)?;
                    wg_configs.remove(tunnel_id);
                    
                    // In a real implementation, bring down the WireGuard interface
                },
                TunnelType::Tls => {
                    // Clean up TLS tunnel
                    // In a real implementation, close TLS connection
                },
                TunnelType::Onion => {
                    // Clean up onion tunnel
                    // In a real implementation, close onion circuit
                },
                _ => {} // Direct tunnels don't need cleanup
            }
            
            info!("Closed tunnel {} to {}", tunnel_id, tunnel.remote_overlay_addr);
            
            Ok(())
        } else {
            Err(NetworkError::Other(format!("Tunnel {} not found", tunnel_id)))
        }
    }
    
    /// Get tunnel info by ID
    pub fn get_tunnel(&self, tunnel_id: &str) -> Result<TunnelInfo> {
        let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
        
        if let Some(tunnel) = tunnels.get(tunnel_id) {
            Ok(tunnel.clone())
        } else {
            Err(NetworkError::Other(format!("Tunnel {} not found", tunnel_id)))
        }
    }
    
    /// Get tunnel stats by ID
    pub fn get_tunnel_stats(&self, tunnel_id: &str) -> Result<TunnelStats> {
        let stats = self.stats.read().map_err(|_| NetworkError::LockError)?;
        
        if let Some(tunnel_stats) = stats.get(tunnel_id) {
            Ok(tunnel_stats.clone())
        } else {
            Err(NetworkError::Other(format!("Tunnel {} not found", tunnel_id)))
        }
    }
    
    /// Get all active tunnels
    pub fn get_active_tunnels(&self) -> Result<Vec<TunnelInfo>> {
        let tunnels = self.tunnels.read().map_err(|_| NetworkError::LockError)?;
        
        Ok(tunnels.values()
            .filter(|t| t.active)
            .cloned()
            .collect())
    }
    
    /// Start tunnel monitoring
    async fn start_tunnel_monitoring(&self) -> Result<()> {
        let tunnels = Arc::clone(&self.tunnels);
        let stats = Arc::clone(&self.stats);
        
        // Spawn a task to monitor tunnels
        tokio::spawn(async move {
            loop {
                // Check each tunnel's status
                let tunnel_ids = {
                    let tunnels_guard = tunnels.read().unwrap();
                    tunnels_guard.keys().cloned().collect::<Vec<_>>()
                };
                
                for tunnel_id in tunnel_ids {
                    let tunnel = {
                        let tunnels_guard = tunnels.read().unwrap();
                        tunnels_guard.get(&tunnel_id).cloned()
                    };
                    
                    if let Some(tunnel) = tunnel {
                        // Check if tunnel is active
                        if tunnel.active {
                            // Check last activity
                            let stats_guard = stats.read().unwrap();
                            if let Some(tunnel_stats) = stats_guard.get(&tunnel_id) {
                                let now = chrono::Utc::now().timestamp();
                                let idle_time = now - tunnel_stats.last_activity;
                                
                                // If tunnel has been idle for too long, consider checking its health
                                if idle_time > 300 { // 5 minutes
                                    debug!("Tunnel {} has been idle for {} seconds", tunnel_id, idle_time);
                                    
                                    // In a real implementation, we would check tunnel health
                                    // and potentially reconnect or close it
                                }
                            }
                        }
                    }
                }
                
                // Check every 60 seconds
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        });
        
        Ok(())
    }
    
    /// Start a handler for a specific tunnel
    async fn start_tunnel_handler(&self, tunnel_id: &str) -> Result<()> {
        let tunnels = Arc::clone(&self.tunnels);
        let stats = Arc::clone(&self.stats);
        let data_tx = self.data_tx.clone();
        let tunnel_id_owned = tunnel_id.to_string();
        
        // Get the tunnel type
        let tunnel_type = {
            let tunnels_guard = tunnels.read().map_err(|_| NetworkError::LockError)?;
            tunnels_guard.get(tunnel_id)
                .map(|t| t.tunnel_type)
                .unwrap_or(TunnelType::Direct)
        };
        
        // Spawn a task to handle the tunnel
        tokio::spawn(async move {
            match tunnel_type {
                TunnelType::WireGuard => {
                    // For WireGuard, we'd monitor the interface
                    debug!("Started WireGuard tunnel handler for {}", tunnel_id_owned);
                    
                    loop {
                        // Simulate activity
                        tokio::time::sleep(Duration::from_secs(10)).await;
                        
                        // Check if tunnel is still active
                        let active = {
                            let tunnels_guard = tunnels.read().unwrap();
                            tunnels_guard.get(&tunnel_id_owned)
                                .map(|t| t.active)
                                .unwrap_or(false)
                        };
                        
                        if !active {
                            debug!("WireGuard tunnel {} is no longer active, stopping handler", tunnel_id_owned);
                            break;
                        }
                        
                        // Update stats to show the tunnel is alive
                        {
                            let mut stats_guard = stats.write().unwrap();
                            if let Some(tunnel_stats) = stats_guard.get_mut(&tunnel_id_owned) {
                                tunnel_stats.last_activity = chrono::Utc::now().timestamp();
                            }
                        }
                    }
                },
                TunnelType::Tls => {
                    // For TLS, we'd maintain the TLS connection
                    debug!("Started TLS tunnel handler for {}", tunnel_id_owned);
                    
                    // Similar to WireGuard but with TLS-specific handling
                },
                TunnelType::Onion => {
                    // For onion routing, we'd maintain the circuit
                    debug!("Started onion tunnel handler for {}", tunnel_id_owned);
                    
                    // Similar to WireGuard but with onion-specific handling
                },
                TunnelType::Direct => {
                    // For direct, there's less to monitor
                    debug!("Started direct tunnel handler for {}", tunnel_id_owned);
                    
                    // Similar but simpler monitoring
                }
            }
        });
        
        Ok(())
    }
} 