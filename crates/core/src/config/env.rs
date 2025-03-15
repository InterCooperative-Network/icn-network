//! Environment variable-based configuration for ICN
//!
//! This module provides a configuration provider that reads configuration from
//! environment variables.

use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use super::{NodeConfig, NetworkConfig, StorageConfig, IdentityConfig, ConfigProvider, ConfigResult, ConfigError};

/// Environment variable prefix for ICN configuration
pub const ENV_PREFIX: &str = "ICN_";

/// An environment variable-based configuration provider
pub struct EnvConfigProvider {
    /// Base configuration to use as fallback
    base_config: NodeConfig,
}

impl EnvConfigProvider {
    /// Create a new environment variable-based configuration provider
    pub fn new() -> Self {
        Self {
            base_config: NodeConfig::default(),
        }
    }
    
    /// Create a new provider with a specific base configuration
    pub fn with_base_config(base_config: NodeConfig) -> Self {
        Self {
            base_config,
        }
    }
    
    /// Parse an environment variable with the ICN prefix
    fn parse_env<T: FromStr>(&self, key: &str, default: T) -> T
    where
        T::Err: std::fmt::Display,
    {
        let env_key = format!("{}{}", ENV_PREFIX, key);
        match env::var(&env_key) {
            Ok(value) => {
                match value.parse::<T>() {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        eprintln!("Warning: Failed to parse env variable {}: {}", env_key, e);
                        default
                    }
                }
            }
            Err(_) => default,
        }
    }
    
    /// Parse a boolean environment variable
    fn parse_bool_env(&self, key: &str, default: bool) -> bool {
        let env_key = format!("{}{}", ENV_PREFIX, key);
        match env::var(&env_key) {
            Ok(value) => {
                match value.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "y" | "on" => true,
                    "false" | "0" | "no" | "n" | "off" => false,
                    _ => {
                        eprintln!("Warning: Failed to parse boolean env variable {}", env_key);
                        default
                    }
                }
            }
            Err(_) => default,
        }
    }
    
    /// Parse a comma-separated list environment variable
    fn parse_list_env(&self, key: &str, default: Vec<String>) -> Vec<String> {
        let env_key = format!("{}{}", ENV_PREFIX, key);
        match env::var(&env_key) {
            Ok(value) => {
                value.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            }
            Err(_) => default,
        }
    }
    
    /// Parse a path environment variable
    fn parse_path_env(&self, key: &str, default: PathBuf) -> PathBuf {
        let env_key = format!("{}{}", ENV_PREFIX, key);
        match env::var(&env_key) {
            Ok(value) => PathBuf::from(value),
            Err(_) => default,
        }
    }
}

#[async_trait::async_trait]
impl ConfigProvider for EnvConfigProvider {
    async fn get_config(&self) -> ConfigResult<NodeConfig> {
        // Create a new configuration based on environment variables
        
        // Network configuration
        let mut network = NetworkConfig {
            host: self.parse_env("NETWORK_HOST", self.base_config.network.host.clone()),
            port: self.parse_env("NETWORK_PORT", self.base_config.network.port),
            bootstrap_nodes: self.parse_list_env("NETWORK_BOOTSTRAP", self.base_config.network.bootstrap_nodes.clone()),
            max_connections: self.parse_env("NETWORK_MAX_CONNECTIONS", self.base_config.network.max_connections),
            connection_timeout: self.parse_env("NETWORK_TIMEOUT", self.base_config.network.connection_timeout),
            heartbeat_interval: self.parse_env("NETWORK_HEARTBEAT", self.base_config.network.heartbeat_interval),
        };
        
        // Storage configuration
        let mut storage = StorageConfig {
            path: self.parse_path_env("STORAGE_PATH", self.base_config.storage.path.clone()),
            sync_writes: self.parse_bool_env("STORAGE_SYNC", self.base_config.storage.sync_writes),
            create_dirs: self.parse_bool_env("STORAGE_CREATE_DIRS", self.base_config.storage.create_dirs),
            use_cache: self.parse_bool_env("STORAGE_CACHE", self.base_config.storage.use_cache),
            max_cache_size: self.parse_env("STORAGE_CACHE_SIZE", self.base_config.storage.max_cache_size),
        };
        
        // Identity configuration
        let mut identity = IdentityConfig {
            key_file: self.parse_path_env("IDENTITY_KEY_FILE", self.base_config.identity.key_file.clone()),
            generate_if_missing: self.parse_bool_env("IDENTITY_GENERATE", self.base_config.identity.generate_if_missing),
            friendly_name: self.parse_env("IDENTITY_NAME", self.base_config.identity.friendly_name.clone()),
        };
        
        // Other configuration
        let environment = self.parse_env("ENVIRONMENT", self.base_config.environment.clone());
        let log_level = self.parse_env("LOG_LEVEL", self.base_config.log_level.clone());
        
        // Custom configuration
        // NOTE: We would need a more sophisticated approach to handle custom configuration via env vars
        // For now, we'll just use the base configuration's custom values
        let custom = self.base_config.custom.clone();
        
        Ok(NodeConfig {
            network,
            storage,
            identity,
            environment,
            log_level,
            custom,
        })
    }
    
    async fn set_config(&self, _config: NodeConfig) -> ConfigResult<()> {
        // Environment variables can't be set by the program in a meaningful way
        Err(ConfigError::InvalidConfig(
            "Cannot set environment variables from within the program".to_string()
        ))
    }
} 