use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use std::net::SocketAddr;
use thiserror::Error;

use crate::error::Result;
use crate::overlay::address::OverlayAddress;
use crate::overlay::TunnelType;

/// Tunnel error
#[derive(Debug, Error)]
pub enum TunnelError {
    #[error("Failed to create tunnel: {0}")]
    CreationFailed(String),
    
    #[error("Tunnel not found: {0}")]
    NotFound(String),
    
    #[error("Tunnel already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Tunnel configuration error: {0}")]
    ConfigurationError(String),
    
    #[error("Tunnel IO error: {0}")]
    IoError(String),
    
    #[error("Tunnel authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Other tunnel error: {0}")]
    Other(String),
}

/// Tunnel status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TunnelStatus {
    /// Tunnel is initializing
    Initializing,
    /// Tunnel is active
    Active,
    /// Tunnel is disconnected
    Disconnected,
    /// Tunnel is in error state
    Error,
}

/// Tunnel statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelStats {
    /// Bytes sent through the tunnel
    pub bytes_sent: u64,
    /// Bytes received through the tunnel
    pub bytes_received: u64,
    /// Packets sent through the tunnel
    pub packets_sent: u64,
    /// Packets received through the tunnel
    pub packets_received: u64,
    /// Last activity timestamp
    pub last_activity: u64,
    /// Current throughput in bytes per second
    pub throughput: u64,
    /// Current latency in milliseconds
    pub latency_ms: u32,
    /// Packet loss percentage
    pub packet_loss: f32,
}

/// WireGuard tunnel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardConfig {
    /// Local private key
    pub private_key: String,
    /// Remote public key
    pub public_key: String,
    /// Pre-shared key (optional)
    pub preshared_key: Option<String>,
    /// Endpoint address
    pub endpoint: SocketAddr,
    /// Allowed IPs
    pub allowed_ips: Vec<String>,
    /// Keep-alive interval in seconds
    pub keepalive: u32,
}

/// Tunnel manager for creating and managing tunnels
pub struct TunnelManager {
    /// Active tunnels
    tunnels: Arc<RwLock<HashMap<String, (TunnelStatus, TunnelStats)>>>,
    /// Local overlay address
    local_address: Option<OverlayAddress>,
}

impl TunnelManager {
    /// Create a new tunnel manager
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
            local_address: None,
        }
    }
    
    /// Initialize with local address
    pub fn initialize(&mut self, local_address: OverlayAddress) {
        self.local_address = Some(local_address);
    }
    
    /// Create a new tunnel
    pub async fn create_tunnel(
        &self,
        tunnel_id: &str,
        tunnel_type: TunnelType,
        remote_addr: &OverlayAddress,
        endpoint: Option<SocketAddr>,
    ) -> Result<TunnelStatus> {
        let mut tunnels = self.tunnels.write().unwrap();
        
        if tunnels.contains_key(tunnel_id) {
            return Err(TunnelError::AlreadyExists(format!("Tunnel already exists: {}", tunnel_id)).into());
        }
        
        let stats = TunnelStats {
            bytes_sent: 0,
            bytes_received: 0,
            packets_sent: 0,
            packets_received: 0,
            last_activity: chrono::Utc::now().timestamp() as u64,
            throughput: 0,
            latency_ms: 0,
            packet_loss: 0.0,
        };
        
        tunnels.insert(tunnel_id.to_string(), (TunnelStatus::Active, stats));
        
        Ok(TunnelStatus::Active)
    }
    
    /// Close a tunnel
    pub async fn close_tunnel(&self, tunnel_id: &str) -> Result<()> {
        let mut tunnels = self.tunnels.write().unwrap();
        
        if !tunnels.contains_key(tunnel_id) {
            return Err(TunnelError::NotFound(format!("Tunnel not found: {}", tunnel_id)).into());
        }
        
        tunnels.remove(tunnel_id);
        
        Ok(())
    }
    
    /// Get tunnel status
    pub async fn get_tunnel_status(&self, tunnel_id: &str) -> Result<TunnelStatus> {
        let tunnels = self.tunnels.read().unwrap();
        
        if let Some((status, _)) = tunnels.get(tunnel_id) {
            Ok(status.clone())
        } else {
            Err(TunnelError::NotFound(format!("Tunnel not found: {}", tunnel_id)).into())
        }
    }
    
    /// Get tunnel statistics
    pub async fn get_tunnel_stats(&self, tunnel_id: &str) -> Result<TunnelStats> {
        let tunnels = self.tunnels.read().unwrap();
        
        if let Some((_, stats)) = tunnels.get(tunnel_id) {
            Ok(stats.clone())
        } else {
            Err(TunnelError::NotFound(format!("Tunnel not found: {}", tunnel_id)).into())
        }
    }
    
    /// Send data through a tunnel
    pub async fn send_data(&self, tunnel_id: &str, data: &[u8]) -> Result<()> {
        let mut tunnels = self.tunnels.write().unwrap();
        
        if let Some((status, stats)) = tunnels.get_mut(tunnel_id) {
            if *status != TunnelStatus::Active {
                return Err(TunnelError::Other(format!("Tunnel not active: {}", tunnel_id)).into());
            }
            
            // Update statistics
            stats.bytes_sent += data.len() as u64;
            stats.packets_sent += 1;
            stats.last_activity = chrono::Utc::now().timestamp() as u64;
            
            Ok(())
        } else {
            Err(TunnelError::NotFound(format!("Tunnel not found: {}", tunnel_id)).into())
        }
    }
    
    /// Get all active tunnels
    pub async fn get_active_tunnels(&self) -> Result<Vec<String>> {
        let tunnels = self.tunnels.read().unwrap();
        
        let active_tunnels = tunnels
            .iter()
            .filter(|(_, (status, _))| *status == TunnelStatus::Active)
            .map(|(id, _)| id.clone())
            .collect();
            
        Ok(active_tunnels)
    }
} 