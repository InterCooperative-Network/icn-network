//! Tunneling module for overlay network
//!
//! This module provides tunneling capabilities for the overlay network,
//! allowing nodes to establish secure connections across network boundaries.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};

/// Tunnel type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TunnelType {
    /// WireGuard tunnel
    WireGuard,
    /// TLS tunnel
    TLS,
    /// IPsec tunnel
    IPsec,
    /// Custom tunnel type
    Custom(String),
}

impl fmt::Display for TunnelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WireGuard => write!(f, "WireGuard"),
            Self::TLS => write!(f, "TLS"),
            Self::IPsec => write!(f, "IPsec"),
            Self::Custom(s) => write!(f, "Custom({})", s),
        }
    }
}

/// WireGuard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WireGuardConfig {
    /// Private key
    pub private_key: String,
    /// Public key
    pub public_key: String,
    /// Endpoint address
    pub endpoint: String,
    /// Allowed IPs
    pub allowed_ips: Vec<String>,
    /// Listen port
    pub listen_port: Option<u16>,
    /// Persistent keepalive
    pub persistent_keepalive: Option<u16>,
}

/// Tunnel information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TunnelInfo {
    /// Tunnel ID
    pub id: String,
    /// Tunnel type
    pub tunnel_type: TunnelType,
    /// Source node ID
    pub source: String,
    /// Destination node ID
    pub destination: String,
    /// Tunnel status
    pub status: TunnelStatus,
    /// Tunnel statistics
    pub stats: TunnelStats,
    /// Creation time
    pub created_at: u64,
    /// Last updated time
    pub updated_at: u64,
}

/// Tunnel status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TunnelStatus {
    /// Initializing
    Initializing,
    /// Connected
    Connected,
    /// Disconnected
    Disconnected,
    /// Error
    Error(String),
}

/// Tunnel statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TunnelStats {
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Packets sent
    pub packets_sent: u64,
    /// Packets received
    pub packets_received: u64,
    /// Last activity timestamp
    pub last_activity: Option<u64>,
}

/// Forwarding policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ForwardingPolicy {
    /// Forward all traffic
    All,
    /// Forward only to specific destinations
    Specific(Vec<String>),
    /// Forward based on a pattern
    Pattern(String),
}

/// Tunnel error
#[derive(Debug, Clone, thiserror::Error)]
pub enum TunnelError {
    /// Initialization error
    #[error("Tunnel initialization error: {0}")]
    InitError(String),
    /// Connection error
    #[error("Tunnel connection error: {0}")]
    ConnectionError(String),
    /// Permission error
    #[error("Tunnel permission error: {0}")]
    PermissionError(String),
    /// Configuration error
    #[error("Tunnel configuration error: {0}")]
    ConfigError(String),
    /// Other error
    #[error("Tunnel error: {0}")]
    Other(String),
}

/// Tunnel manager for handling overlay network tunnels
#[derive(Debug)]
pub struct TunnelManager {
    /// Active tunnels
    tunnels: Arc<RwLock<HashMap<String, TunnelInfo>>>,
}

impl TunnelManager {
    /// Create a new tunnel manager
    pub fn new() -> Self {
        Self {
            tunnels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new tunnel
    pub async fn create_tunnel(&self, info: TunnelInfo) -> Result<String, TunnelError> {
        let id = info.id.clone();
        let mut tunnels = self.tunnels.write().await;
        tunnels.insert(id.clone(), info);
        Ok(id)
    }

    /// Get tunnel information
    pub async fn get_tunnel(&self, id: &str) -> Option<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.get(id).cloned()
    }

    /// Update tunnel status
    pub async fn update_status(&self, id: &str, status: TunnelStatus) -> Result<(), TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        if let Some(info) = tunnels.get_mut(id) {
            info.status = status;
            info.updated_at = chrono::Utc::now().timestamp() as u64;
            Ok(())
        } else {
            Err(TunnelError::Other(format!("Tunnel not found: {}", id)))
        }
    }

    /// Update tunnel statistics
    pub async fn update_stats(&self, id: &str, stats: TunnelStats) -> Result<(), TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        if let Some(info) = tunnels.get_mut(id) {
            info.stats = stats;
            info.updated_at = chrono::Utc::now().timestamp() as u64;
            Ok(())
        } else {
            Err(TunnelError::Other(format!("Tunnel not found: {}", id)))
        }
    }

    /// Close a tunnel
    pub async fn close_tunnel(&self, id: &str) -> Result<(), TunnelError> {
        let mut tunnels = self.tunnels.write().await;
        if tunnels.remove(id).is_some() {
            Ok(())
        } else {
            Err(TunnelError::Other(format!("Tunnel not found: {}", id)))
        }
    }

    /// Get all tunnels
    pub async fn get_all_tunnels(&self) -> Vec<TunnelInfo> {
        let tunnels = self.tunnels.read().await;
        tunnels.values().cloned().collect()
    }
} 