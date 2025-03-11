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

    /// Federation identifier
    pub federation_id: String,

    /// Node capabilities configuration
    pub capabilities: NodeCapabilitiesConfig,

    /// Federation configuration
    pub federation: FederationConfig,
}

/// Node capabilities configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilitiesConfig {
    /// Enable storage capability
    pub storage: bool,

    /// Enable compute capability
    pub compute: bool,

    /// Enable gateway capability
    pub gateway: bool,

    /// Maximum storage space in GB
    pub max_storage_gb: Option<u64>,

    /// Maximum CPU cores to use
    pub max_cpu_cores: Option<u32>,

    /// Maximum memory to use in MB
    pub max_memory_mb: Option<u64>,
}

/// Federation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Name of the federation
    pub name: String,

    /// Description of the federation
    pub description: Option<String>,

    /// Federation discovery seeds
    pub discovery_seeds: Vec<String>,

    /// Trust configuration
    pub trust: FederationTrustConfig,
}

/// Federation trust configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationTrustConfig {
    /// Minimum trust score for peer federation
    pub min_trust_score: f64,

    /// Trust decay rate
    pub trust_decay_rate: f64,

    /// Required endorsements for trust elevation
    pub required_endorsements: u32,
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
            federation_id: "fed-1".to_string(),
            capabilities: NodeCapabilitiesConfig::default(),
            federation: FederationConfig::default(),
        }
    }
}

impl Default for NodeCapabilitiesConfig {
    fn default() -> Self {
        Self {
            storage: false,
            compute: false,
            gateway: false,
            max_storage_gb: None,
            max_cpu_cores: None,
            max_memory_mb: None,
        }
    }
}

impl Default for FederationConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            description: None,
            discovery_seeds: Vec::new(),
            trust: FederationTrustConfig::default(),
        }
    }
}

impl Default for FederationTrustConfig {
    fn default() -> Self {
        Self {
            min_trust_score: 0.5,
            trust_decay_rate: 0.01,
            required_endorsements: 3,
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

        // Ensure node_id is not empty
        if self.node_id.is_none() {
            return Err(Error::configuration("Node ID cannot be empty"));
        }

        // Ensure federation_id is not empty
        if self.federation_id.is_empty() {
            return Err(Error::configuration("Federation ID cannot be empty"));
        }

        // Validate storage limits if storage is enabled
        if self.capabilities.storage {
            if self.capabilities.max_storage_gb.is_none() {
                return Err(Error::configuration("max_storage_gb must be set when storage is enabled"));
            }
        }

        // Validate compute limits if compute is enabled
        if self.capabilities.compute {
            if self.capabilities.max_cpu_cores.is_none() || self.capabilities.max_memory_mb.is_none() {
                return Err(Error::configuration("max_cpu_cores and max_memory_mb must be set when compute is enabled"));
            }
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
