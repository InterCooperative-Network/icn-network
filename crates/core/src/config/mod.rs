use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors that can occur in configuration operations
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),
    
    #[error("Invalid value for environment variable {0}: {1}")]
    InvalidEnvVar(String, String),
    
    #[error("File not found: {0}")]
    FileNotFound(String),
    
    #[error("Failed to read file: {0}")]
    FileReadError(String),
    
    #[error("Failed to parse YAML: {0}")]
    YamlParseError(#[from] serde_yaml::Error),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;

/// TLS Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub cert_file: String,
    pub key_file: String,
    pub ca_file: String,
    #[serde(default = "default_verify_client")]
    pub verify_client: bool,
    #[serde(default = "default_verify_hostname")]
    pub verify_hostname: bool,
}

fn default_verify_client() -> bool {
    true
}

fn default_verify_hostname() -> bool {
    true
}

/// Main Node Configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub node_id: String,
    pub coop_id: String,
    pub node_type: String,
    pub listen_addr: String,
    #[serde(default)]
    pub peers: Vec<String>,
    #[serde(default = "default_discovery_interval")]
    pub discovery_interval: u64,
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval: u64,
    pub data_dir: String,
    #[serde(default = "default_cert_dir")]
    pub cert_dir: String,
    #[serde(default = "default_log_dir")]
    pub log_dir: String,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    pub tls: TlsConfig,
}

fn default_discovery_interval() -> u64 {
    30
}

fn default_health_check_interval() -> u64 {
    10
}

fn default_cert_dir() -> String {
    "/etc/icn/certs".to_string()
}

fn default_log_dir() -> String {
    "/var/log/icn".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl NodeConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Result<Self> {
        // Check if there's a configuration file path in the environment
        let config_path = env::var("ICN_CONFIG_FILE").unwrap_or_else(|_| "/etc/icn/node.yaml".to_string());
        
        if Path::new(&config_path).exists() {
            return Self::from_file(&config_path);
        }
        
        // Otherwise, build config from environment variables
        let node_id = env::var("ICN_NODE_ID")
            .map_err(|e| ConfigError::EnvVarNotFound(format!("ICN_NODE_ID: {}", e)))?;
            
        let coop_id = env::var("ICN_COOP_ID")
            .map_err(|e| ConfigError::EnvVarNotFound(format!("ICN_COOP_ID: {}", e)))?;
            
        let node_type = env::var("ICN_NODE_TYPE").unwrap_or_else(|_| "primary".to_string());
        let listen_addr = env::var("ICN_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:9000".to_string());
        
        // Parse peers if provided
        let peers_str = env::var("ICN_PEERS").unwrap_or_else(|_| "[]".to_string());
        let peers: Vec<String> = if peers_str.starts_with('[') && peers_str.ends_with(']') {
            // Try to parse as JSON array
            serde_json::from_str(&peers_str)
                .map_err(|e| ConfigError::InvalidEnvVar("ICN_PEERS".to_string(), e.to_string()))?
        } else {
            // Try to parse as comma-separated list
            peers_str.split(',').map(|s| s.trim().to_string()).collect()
        };
        
        let discovery_interval = env::var("ICN_DISCOVERY_INTERVAL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or_else(default_discovery_interval);
            
        let health_check_interval = env::var("ICN_HEALTH_CHECK_INTERVAL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or_else(default_health_check_interval);
            
        let data_dir = env::var("ICN_DATA_DIR").unwrap_or_else(|_| "/var/lib/icn".to_string());
        let cert_dir = env::var("ICN_CERT_DIR").unwrap_or_else(|_| default_cert_dir());
        let log_dir = env::var("ICN_LOG_DIR").unwrap_or_else(|_| default_log_dir());
        let log_level = env::var("ICN_LOG_LEVEL").unwrap_or_else(|_| default_log_level());
        
        // TLS configuration
        let tls_enabled = env::var("ICN_TLS_ENABLED")
            .ok()
            .and_then(|v| v.parse::<bool>().ok())
            .unwrap_or(true);
            
        let tls_config = TlsConfig {
            enabled: tls_enabled,
            cert_file: format!("{}/node.crt", cert_dir),
            key_file: format!("{}/node.key", cert_dir),
            ca_file: format!("{}/ca.crt", cert_dir),
            verify_client: env::var("ICN_VERIFY_CLIENT")
                .ok()
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or_else(default_verify_client),
            verify_hostname: env::var("ICN_VERIFY_HOSTNAME")
                .ok()
                .and_then(|v| v.parse::<bool>().ok())
                .unwrap_or_else(default_verify_hostname),
        };
        
        Ok(NodeConfig {
            node_id,
            coop_id,
            node_type,
            listen_addr,
            peers,
            discovery_interval,
            health_check_interval,
            data_dir,
            cert_dir,
            log_dir,
            log_level,
            tls: tls_config,
        })
    }
    
    /// Load configuration from file
    pub fn from_file(path: &str) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .map_err(|e| ConfigError::FileReadError(format!("Failed to read {}: {}", path, e)))?;
            
        let config: NodeConfig = serde_yaml::from_str(&contents)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = NodeConfig::from_env().unwrap();
        assert_eq!(config.node_type, "primary");
        assert_eq!(config.listen_addr, "0.0.0.0:9000");
        assert_eq!(config.discovery_interval, 30);
        assert_eq!(config.health_check_interval, 10);
        assert_eq!(config.cert_dir, "/etc/icn/certs");
        assert_eq!(config.log_dir, "/var/log/icn");
        assert_eq!(config.log_level, "info");
    }
    
    #[test]
    fn test_tls_config() {
        let config = NodeConfig::from_env().unwrap();
        assert!(config.tls.enabled);
        assert!(config.tls.verify_client);
        assert!(config.tls.verify_hostname);
        assert!(config.tls.cert_file.ends_with("node.crt"));
        assert!(config.tls.key_file.ends_with("node.key"));
        assert!(config.tls.ca_file.ends_with("ca.crt"));
    }
}
