//! Node configuration

use icn_common::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
    pub node_id: Option<String>,
    
    /// Node name
    pub name: String,
    
    /// Data directory path
    pub data_dir: String,
    
    /// Network mode
    #[serde(default)]
    pub network_mode: NetworkMode,
    
    /// Listen address for network connections
    pub listen_address: Option<String>,
    
    /// Bootstrap peers
    #[serde(default)]
    pub bootstrap_peers: Vec<String>,
    
    /// Enable identity system
    #[serde(default = "default_true")]
    pub enable_identity: bool,
    
    /// Enable governance system
    #[serde(default = "default_true")]
    pub enable_governance: bool,
    
    /// Enable economic system
    #[serde(default = "default_true")]
    pub enable_economic: bool,
    
    /// Enable resource system
    #[serde(default = "default_false")]
    pub enable_resource: bool,
    
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
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

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: None,
            name: "icn-node".to_string(),
            data_dir: "./data".to_string(),
            network_mode: NetworkMode::default(),
            listen_address: Some("127.0.0.1:9000".to_string()),
            bootstrap_peers: vec![],
            enable_identity: true,
            enable_governance: true,
            enable_economic: true,
            enable_resource: false,
            log_level: default_log_level(),
        }
    }
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
    
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(Error::configuration("Node name cannot be empty"));
        }
        
        // Ensure data directory is valid
        if self.data_dir.is_empty() {
            return Err(Error::configuration("Data directory cannot be empty"));
        }
        
        Ok(())
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
