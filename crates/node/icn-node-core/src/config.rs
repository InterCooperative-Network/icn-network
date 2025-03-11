//! Node configuration

use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use icn_common::config::Config;
use icn_storage_system::StorageOptions;

/// Network operation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NetworkMode {
    /// Standalone mode (no networking)
    Standalone,
    /// Local network mode
    Local,
    /// Mesh network mode
    Mesh,
    /// Full network mode
    Full,
    /// Relay mode for federation gateway
    Relay,
}

impl Default for NetworkMode {
    fn default() -> Self {
        NetworkMode::Local
    }
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Node identifier
    pub node_id: String,
    
    /// Federation identifier
    pub federation_id: String,
    
    /// Federation endpoints
    pub federation_endpoints: Vec<String>,
    
    /// Storage configuration
    pub storage: StorageOptions,
    
    /// Node capabilities configuration
    pub capabilities: CapabilitiesConfig,
    
    /// Network configuration
    pub network: NetworkConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitiesConfig {
    /// Whether storage capability is enabled
    pub storage_enabled: bool,
    
    /// Whether compute capability is enabled
    pub compute_enabled: bool,
    
    /// Whether gateway capability is enabled
    pub gateway_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// P2P listening address
    pub p2p_addr: String,
    
    /// API listening address
    pub api_addr: String,
    
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: "node-1".to_string(),
            federation_id: "local".to_string(),
            federation_endpoints: Vec::new(),
            storage: StorageOptions::default(),
            capabilities: CapabilitiesConfig::default(),
            network: NetworkConfig::default(),
        }
    }
}

impl Default for CapabilitiesConfig {
    fn default() -> Self {
        Self {
            storage_enabled: false,
            compute_enabled: false,
            gateway_enabled: false,
        }
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            p2p_addr: "/ip4/0.0.0.0/tcp/9000".to_string(),
            api_addr: "127.0.0.1:8000".to_string(),
            bootstrap_nodes: Vec::new(),
        }
    }
}

impl Config for NodeConfig {
    fn validate(&self) -> icn_common::Result<()> {
        // Validate node ID
        if self.node_id.is_empty() {
            return Err(icn_common::Error::validation("Node ID cannot be empty"));
        }

        // Validate federation ID
        if self.federation_id.is_empty() {
            return Err(icn_common::Error::validation("Federation ID cannot be empty"));
        }

        // Validate network addresses
        if self.network.p2p_addr.is_empty() {
            return Err(icn_common::Error::validation("P2P address cannot be empty"));
        }
        if self.network.api_addr.is_empty() {
            return Err(icn_common::Error::validation("API address cannot be empty")); 
        }

        Ok(())
    }
}

fn default_true() -> bool {
    true
}

fn default_false() -> bool {
    false
}

fn default_log_level() -> String {
    "info".to_string()
}

impl NodeConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::configuration(format!("Failed to read config file: {}", e)))?;
            
        let config: NodeConfig = toml::from_str(&content)
            .map_err(|e| Error::configuration(format!("Failed to parse config file: {}", e)))?;
            
        Ok(config)
    }
    
    /// Save configuration to a TOML file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::configuration(format!("Failed to serialize config: {}", e)))?;
            
        fs::write(path, content)
            .map_err(|e| Error::configuration(format!("Failed to write config file: {}", e)))?;
            
        Ok(())
    }
}
