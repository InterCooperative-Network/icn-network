//! Network configuration module
//!
//! This module provides configuration types and utilities for the network layer.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use libp2p::Multiaddr;
use serde::{Deserialize, Serialize};

/// Dual-stack address configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualStackConfig {
    /// IPv4 configuration
    pub ipv4: Option<Ipv4Config>,
    /// IPv6 configuration
    pub ipv6: Option<Ipv6Config>,
    /// Prefer IPv6 connections when available
    #[serde(default = "default_prefer_ipv6")]
    pub prefer_ipv6: bool,
}

impl Default for DualStackConfig {
    fn default() -> Self {
        Self {
            ipv4: Some(Ipv4Config::default()),
            ipv6: Some(Ipv6Config::default()),
            prefer_ipv6: true,
        }
    }
}

impl DualStackConfig {
    /// Create a new dual-stack configuration
    pub fn new(ipv4: Option<Ipv4Config>, ipv6: Option<Ipv6Config>, prefer_ipv6: bool) -> Self {
        Self {
            ipv4,
            ipv6,
            prefer_ipv6,
        }
    }
    
    /// Get multiaddresses for listening based on the configuration
    pub fn get_listen_addresses(&self) -> Vec<Multiaddr> {
        let mut addrs = Vec::new();
        
        if let Some(ipv6) = &self.ipv6 {
            // Add IPv6 addresses first if preferred
            if self.prefer_ipv6 {
                addrs.extend(ipv6.to_multiaddresses());
            }
        }
        
        if let Some(ipv4) = &self.ipv4 {
            // Add IPv4 addresses
            addrs.extend(ipv4.to_multiaddresses());
        }
        
        if let Some(ipv6) = &self.ipv6 {
            // Add IPv6 addresses last if not preferred
            if !self.prefer_ipv6 {
                addrs.extend(ipv6.to_multiaddresses());
            }
        }
        
        addrs
    }
    
    /// Determine if IPv6 is enabled
    pub fn is_ipv6_enabled(&self) -> bool {
        self.ipv6.is_some()
    }
    
    /// Determine if IPv4 is enabled
    pub fn is_ipv4_enabled(&self) -> bool {
        self.ipv4.is_some()
    }
}

/// IPv4 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv4Config {
    /// IPv4 address to bind to
    #[serde(default = "default_ipv4_address")]
    pub address: Ipv4Addr,
    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,
    /// Interface to bind to
    pub interface: Option<String>,
}

impl Default for Ipv4Config {
    fn default() -> Self {
        Self {
            address: default_ipv4_address(),
            port: default_port(),
            interface: None,
        }
    }
}

impl Ipv4Config {
    /// Create a new IPv4 configuration
    pub fn new(address: Ipv4Addr, port: u16, interface: Option<String>) -> Self {
        Self {
            address,
            port,
            interface,
        }
    }
    
    /// Convert to multiaddresses
    pub fn to_multiaddresses(&self) -> Vec<Multiaddr> {
        vec![format!("/ip4/{}/tcp/{}", self.address, self.port)
            .parse()
            .expect("Invalid IPv4 multiaddress")]
    }
    
    /// Get socket address
    pub fn to_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(self.address), self.port)
    }
}

/// IPv6 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ipv6Config {
    /// IPv6 address to bind to
    #[serde(default = "default_ipv6_address")]
    pub address: Ipv6Addr,
    /// Port to bind to
    #[serde(default = "default_port")]
    pub port: u16,
    /// Interface to bind to
    pub interface: Option<String>,
}

impl Default for Ipv6Config {
    fn default() -> Self {
        Self {
            address: default_ipv6_address(),
            port: default_port(),
            interface: None,
        }
    }
}

impl Ipv6Config {
    /// Create a new IPv6 configuration
    pub fn new(address: Ipv6Addr, port: u16, interface: Option<String>) -> Self {
        Self {
            address,
            port,
            interface,
        }
    }
    
    /// Convert to multiaddresses
    pub fn to_multiaddresses(&self) -> Vec<Multiaddr> {
        vec![format!("/ip6/{}/tcp/{}", self.address, self.port)
            .parse()
            .expect("Invalid IPv6 multiaddress")]
    }
    
    /// Get socket address
    pub fn to_socket_addr(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V6(self.address), self.port)
    }
}

/// Default values for configuration
fn default_ipv4_address() -> Ipv4Addr {
    Ipv4Addr::UNSPECIFIED
}

fn default_ipv6_address() -> Ipv6Addr {
    Ipv6Addr::UNSPECIFIED
}

fn default_port() -> u16 {
    9000
}

fn default_prefer_ipv6() -> bool {
    true
}

/// Network transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    /// Dual-stack configuration
    #[serde(default)]
    pub dual_stack: DualStackConfig,
    
    /// QUIC configuration
    #[serde(default)]
    pub quic: QuicConfig,
    
    /// WebRTC configuration
    #[serde(default)]
    pub webrtc: WebRtcConfig,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            dual_stack: DualStackConfig::default(),
            quic: QuicConfig::default(),
            webrtc: WebRtcConfig::default(),
        }
    }
}

/// QUIC transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuicConfig {
    /// Enable QUIC transport
    #[serde(default = "default_true")]
    pub enabled: bool,
    
    /// Path to certificate file
    pub cert_path: Option<PathBuf>,
    
    /// Path to key file
    pub key_path: Option<PathBuf>,
    
    /// Keep alive interval in seconds
    #[serde(default = "default_keep_alive")]
    pub keep_alive_interval_secs: u64,
}

impl Default for QuicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cert_path: None,
            key_path: None,
            keep_alive_interval_secs: default_keep_alive(),
        }
    }
}

/// WebRTC transport configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRtcConfig {
    /// Enable WebRTC transport
    #[serde(default = "default_false")]
    pub enabled: bool,
    
    /// STUN servers for NAT traversal
    #[serde(default)]
    pub stun_servers: Vec<String>,
    
    /// TURN servers for NAT traversal
    #[serde(default)]
    pub turn_servers: Vec<TurnServer>,
}

impl Default for WebRtcConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: Vec::new(),
        }
    }
}

/// TURN server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnServer {
    /// Server URL
    pub url: String,
    /// Username for authentication
    pub username: Option<String>,
    /// Credential for authentication
    pub credential: Option<String>,
}

/// Network discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Enable mDNS discovery
    #[serde(default = "default_true")]
    pub enable_mdns: bool,
    
    /// Enable Kademlia DHT
    #[serde(default = "default_true")]
    pub enable_kademlia: bool,
    
    /// Bootstrap peers
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
    
    /// Discovery interval in seconds
    #[serde(default = "default_discovery_interval")]
    pub discovery_interval_secs: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            enable_mdns: true,
            enable_kademlia: true,
            bootstrap_peers: Vec::new(),
            discovery_interval_secs: default_discovery_interval(),
        }
    }
}

/// Default boolean values
fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

/// Default time values
fn default_keep_alive() -> u64 {
    30
}

fn default_discovery_interval() -> u64 {
    60
}

/// Network metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics collection
    #[serde(default = "default_false")]
    pub enabled: bool,
    
    /// Prometheus metrics endpoint
    pub prometheus_endpoint: Option<String>,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            prometheus_endpoint: Some("0.0.0.0:9090".to_string()),
        }
    }
}

/// Combined network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Transport configuration
    #[serde(default)]
    pub transport: TransportConfig,
    
    /// Discovery configuration
    #[serde(default)]
    pub discovery: DiscoveryConfig,
    
    /// Metrics configuration
    #[serde(default)]
    pub metrics: MetricsConfig,
    
    /// Path to peer store
    pub peer_store_path: Option<PathBuf>,
    
    /// Enable circuit relay
    #[serde(default = "default_true")]
    pub enable_circuit_relay: bool,
    
    /// Maximum connections per peer
    #[serde(default = "default_max_connections")]
    pub max_connections_per_peer: u32,
    
    /// Connection timeout in seconds
    #[serde(default = "default_connection_timeout")]
    pub connection_timeout_secs: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            transport: TransportConfig::default(),
            discovery: DiscoveryConfig::default(),
            metrics: MetricsConfig::default(),
            peer_store_path: None,
            enable_circuit_relay: true,
            max_connections_per_peer: default_max_connections(),
            connection_timeout_secs: default_connection_timeout(),
        }
    }
}

impl NetworkConfig {
    /// Load configuration from a file
    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config = serde_json::from_str(&content)?;
        Ok(config)
    }
    
    /// Get a list of all listen addresses
    pub fn get_listen_addresses(&self) -> Vec<Multiaddr> {
        self.transport.dual_stack.get_listen_addresses()
    }
    
    /// Get connection timeout as Duration
    pub fn connection_timeout(&self) -> Duration {
        Duration::from_secs(self.connection_timeout_secs)
    }
}

/// More default values
fn default_max_connections() -> u32 {
    50
}

fn default_connection_timeout() -> u64 {
    30
}

/// Main network service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkServiceConfig {
    /// Network configuration
    #[serde(default)]
    pub network: NetworkConfig,
    
    /// Node identity
    pub node_id: String,
    
    /// Node type
    #[serde(default = "default_node_type")]
    pub node_type: String,
}

impl Default for NetworkServiceConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            node_id: "node-default".to_string(),
            node_type: default_node_type(),
        }
    }
}

fn default_node_type() -> String {
    "standard".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_dual_stack_config() {
        let config = DualStackConfig::default();
        assert!(config.prefer_ipv6);
        assert!(config.is_ipv6_enabled());
        assert!(config.is_ipv4_enabled());
        
        let addrs = config.get_listen_addresses();
        assert_eq!(addrs.len(), 2);
        
        // IPv6 should be first because prefer_ipv6 is true
        let addr_str = addrs[0].to_string();
        assert!(addr_str.contains("/ip6/::"));
    }
    
    #[test]
    fn test_ipv4_only() {
        let config = DualStackConfig::new(Some(Ipv4Config::default()), None, false);
        assert!(!config.prefer_ipv6);
        assert!(!config.is_ipv6_enabled());
        assert!(config.is_ipv4_enabled());
        
        let addrs = config.get_listen_addresses();
        assert_eq!(addrs.len(), 1);
        
        let addr_str = addrs[0].to_string();
        assert!(addr_str.contains("/ip4/0.0.0.0"));
    }
    
    #[test]
    fn test_ipv6_only() {
        let config = DualStackConfig::new(None, Some(Ipv6Config::default()), true);
        assert!(config.prefer_ipv6);
        assert!(config.is_ipv6_enabled());
        assert!(!config.is_ipv4_enabled());
        
        let addrs = config.get_listen_addresses();
        assert_eq!(addrs.len(), 1);
        
        let addr_str = addrs[0].to_string();
        assert!(addr_str.contains("/ip6/::"));
    }
} 