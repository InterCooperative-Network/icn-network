//! Configuration for ICN
//!
//! This module provides configuration utilities and types for ICN components.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::fs;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use tracing::{debug, error, info, warn};

/// Error types for configuration operations
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Parsing error
    #[error("Parsing error: {0}")]
    ParseError(String),
    
    /// Key not found
    #[error("Configuration key not found: {0}")]
    KeyNotFound(String),
    
    /// Value error
    #[error("Invalid value for key {0}: {1}")]
    InvalidValue(String, String),
}

/// Result type for configuration operations
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Host to bind to
    pub host: String,
    /// Port to bind to
    pub port: u16,
    /// Bootstrap nodes
    pub bootstrap_nodes: Vec<String>,
    /// Maximum number of connections
    pub max_connections: usize,
    /// Connection timeout in seconds
    pub connection_timeout: u64,
    /// Heartbeat interval in seconds
    pub heartbeat_interval: u64,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 9000,
            bootstrap_nodes: Vec::new(),
            max_connections: 50,
            connection_timeout: 5,
            heartbeat_interval: 30,
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to storage directory
    pub path: PathBuf,
    /// Whether to sync writes immediately
    pub sync_writes: bool,
    /// Whether to create directories if they don't exist
    pub create_dirs: bool,
    /// Whether to use caching
    pub use_cache: bool,
    /// Maximum cache size in bytes
    pub max_cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            path: PathBuf::from("data"),
            sync_writes: true,
            create_dirs: true,
            use_cache: true,
            max_cache_size: 104_857_600, // 100 MB
        }
    }
}

/// Identity configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityConfig {
    /// Path to identity key file
    pub key_file: PathBuf,
    /// Generate a new identity if one doesn't exist
    pub generate_if_missing: bool,
    /// Friendly name for this node
    pub friendly_name: String,
}

impl Default for IdentityConfig {
    fn default() -> Self {
        Self {
            key_file: PathBuf::from("identity.key"),
            generate_if_missing: true,
            friendly_name: "ICN Node".to_string(),
        }
    }
}

/// Main configuration for an ICN node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Network configuration
    pub network: NetworkConfig,
    /// Storage configuration
    pub storage: StorageConfig,
    /// Identity configuration
    pub identity: IdentityConfig,
    /// Environment (e.g., "development", "production")
    pub environment: String,
    /// Log level
    pub log_level: String,
    /// Additional custom configuration
    pub custom: HashMap<String, serde_json::Value>,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            network: NetworkConfig::default(),
            storage: StorageConfig::default(),
            identity: IdentityConfig::default(),
            environment: "development".to_string(),
            log_level: "info".to_string(),
            custom: HashMap::new(),
        }
    }
}

impl NodeConfig {
    /// Load configuration from a TOML file
    pub async fn from_file<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let content = fs::read_to_string(path).await
            .map_err(|e| ConfigError::IoError(e))?;
        
        let config: Self = toml::from_str(&content)
            .map_err(|e| ConfigError::ParseError(format!("Failed to parse config: {}", e)))?;
        
        Ok(config)
    }
    
    /// Save configuration to a TOML file
    pub async fn save_to_file<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::ParseError(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(path, content).await
            .map_err(|e| ConfigError::IoError(e))?;
        
        Ok(())
    }
    
    /// Get a custom value by key
    pub fn get_custom<T: for<'de> Deserialize<'de>>(&self, key: &str) -> ConfigResult<T> {
        let value = self.custom.get(key)
            .ok_or_else(|| ConfigError::KeyNotFound(key.to_string()))?;
        
        serde_json::from_value(value.clone())
            .map_err(|e| ConfigError::InvalidValue(
                key.to_string(),
                format!("Failed to deserialize value: {}", e)
            ))
    }
    
    /// Set a custom value by key
    pub fn set_custom<T: Serialize>(&mut self, key: &str, value: T) -> ConfigResult<()> {
        let json_value = serde_json::to_value(value)
            .map_err(|e| ConfigError::InvalidValue(
                key.to_string(),
                format!("Failed to serialize value: {}", e)
            ))?;
        
        self.custom.insert(key.to_string(), json_value);
        Ok(())
    }
}

/// A configuration provider interface
#[async_trait::async_trait]
pub trait ConfigProvider: Send + Sync {
    /// Get configuration
    async fn get_config(&self) -> ConfigResult<NodeConfig>;
    
    /// Set configuration
    async fn set_config(&self, config: NodeConfig) -> ConfigResult<()>;
}

/// A file-based configuration provider
pub struct FileConfigProvider {
    /// Path to the configuration file
    config_path: PathBuf,
    /// Cached configuration
    config: Arc<RwLock<Option<NodeConfig>>>,
}

impl FileConfigProvider {
    /// Create a new file-based configuration provider
    pub fn new<P: AsRef<Path>>(config_path: P) -> Self {
        Self {
            config_path: config_path.as_ref().to_path_buf(),
            config: Arc::new(RwLock::new(None)),
        }
    }
}

#[async_trait::async_trait]
impl ConfigProvider for FileConfigProvider {
    async fn get_config(&self) -> ConfigResult<NodeConfig> {
        // Try to get from cache first
        {
            let config = self.config.read().await;
            if let Some(config) = config.as_ref() {
                return Ok(config.clone());
            }
        }
        
        // Load from file
        let config = if self.config_path.exists() {
            NodeConfig::from_file(&self.config_path).await?
        } else {
            // Create default configuration if file doesn't exist
            let config = NodeConfig::default();
            config.save_to_file(&self.config_path).await?;
            config
        };
        
        // Update cache
        {
            let mut cache = self.config.write().await;
            *cache = Some(config.clone());
        }
        
        Ok(config)
    }
    
    async fn set_config(&self, config: NodeConfig) -> ConfigResult<()> {
        // Save to file
        config.save_to_file(&self.config_path).await?;
        
        // Update cache
        {
            let mut cache = self.config.write().await;
            *cache = Some(config);
        }
        
        Ok(())
    }
}

pub mod env;
pub mod command_line; 