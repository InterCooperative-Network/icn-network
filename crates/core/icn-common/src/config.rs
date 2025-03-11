//! Common configuration utilities
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
    net::SocketAddr,
};
use crate::{error::Result, types::NodeType};

/// Base trait for all configuration types
pub trait Configuration: Serialize + for<'de> Deserialize<'de> + Default {
    /// Validate the configuration
    fn validate(&self) -> Result<()>;
    
    /// Load configuration from a file
    fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::configuration(format!("Failed to read config file: {}", e)))?;
        
        Self::from_str(&content)
    }
    
    /// Load configuration from a string
    fn from_str(content: &str) -> Result<Self> {
        let config = toml::from_str(content)
            .map_err(|e| Error::configuration(format!("Failed to parse config: {}", e)))?;
        
        Ok(config)
    }
    
    /// Save configuration to a file
    fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| Error::configuration(format!("Failed to serialize config: {}", e)))?;
        
        fs::write(path, content)
            .map_err(|e| Error::configuration(format!("Failed to write config file: {}", e)))?;
        
        Ok(())
    }

    /// Load configuration from environment variables with the given prefix
    fn from_env(prefix: &str) -> Result<Self> {
        let mut config = Self::default();
        config.validate()?;
        Ok(config)
    }
}

/// Environment configuration, used to provide runtime settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    /// Application environment (development, testing, production)
    #[serde(default = "default_environment")]
    pub environment: String,
    
    /// Log level
    #[serde(default = "default_log_level")]
    pub log_level: String,
    
    /// Enable debug features
    #[serde(default)]
    pub debug: bool,
}

fn default_environment() -> String {
    "development".to_string()
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for Environment {
    fn default() -> Self {
        Self {
            environment: default_environment(),
            log_level: default_log_level(),
            debug: false,
        }
    }
}

impl Configuration for Environment {
    fn validate(&self) -> Result<()> {
        // Validate environment
        match self.environment.as_str() {
            "development" | "testing" | "production" => {},
            _ => return Err(Error::configuration(format!(
                "Invalid environment: {}", self.environment
            ))),
        }
        
        // Validate log level
        match self.log_level.as_str() {
            "trace" | "debug" | "info" | "warn" | "error" => {},
            _ => return Err(Error::configuration(format!(
                "Invalid log level: {}", self.log_level
            ))),
        }
        
        Ok(())
    }
}

/// Helper function to create directories needed for configurations
pub fn ensure_directory(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| Error::configuration(format!(
                "Failed to create directory '{}': {}", 
                path.display(), e
            )))?;
    }
    
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    // Node identity
    pub node_id: String,
    pub coop_id: String,
    pub node_type: NodeType,

    // Network configuration
    pub listen_addr: SocketAddr,
    pub peers: Vec<SocketAddr>,
    pub discovery_interval: Option<Duration>,
    pub health_check_interval: Option<Duration>,

    // Storage configuration
    pub data_dir: PathBuf,
    pub cert_dir: PathBuf,

    // Logging configuration
    pub log_dir: PathBuf,
    pub log_level: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: "node-0".to_string(),
            coop_id: "coop-0".to_string(),
            node_type: NodeType::Primary,
            listen_addr: "127.0.0.1:9000".parse().unwrap(),
            peers: Vec::new(),
            discovery_interval: Some(Duration::from_secs(30)),
            health_check_interval: Some(Duration::from_secs(10)),
            data_dir: PathBuf::from("/var/lib/icn"),
            cert_dir: PathBuf::from("/etc/icn/certs"),
            log_dir: PathBuf::from("/var/log/icn"),
            log_level: "info".to_string(),
        }
    }
}

impl NodeConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: NodeConfig = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn from_env() -> Result<Self> {
        let mut config = Self::default();

        if let Ok(node_id) = std::env::var("ICN_NODE_ID") {
            config.node_id = node_id;
        }

        if let Ok(coop_id) = std::env::var("ICN_COOP_ID") {
            config.coop_id = coop_id;
        }

        if let Ok(node_type) = std::env::var("ICN_NODE_TYPE") {
            config.node_type = match node_type.to_lowercase().as_str() {
                "primary" => NodeType::Primary,
                "secondary" => NodeType::Secondary,
                "edge" => NodeType::Edge,
                _ => return Err("Invalid node type".into()),
            };
        }

        if let Ok(addr) = std::env::var("ICN_LISTEN_ADDR") {
            config.listen_addr = addr.parse()?;
        }

        if let Ok(peers) = std::env::var("ICN_PEERS") {
            config.peers = peers
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.parse())
                .collect::<std::result::Result<_, _>>()?;
        }

        if let Ok(interval) = std::env::var("ICN_DISCOVERY_INTERVAL") {
            config.discovery_interval = Some(Duration::from_secs(interval.parse()?));
        }

        if let Ok(interval) = std::env::var("ICN_HEALTH_CHECK_INTERVAL") {
            config.health_check_interval = Some(Duration::from_secs(interval.parse()?));
        }

        if let Ok(dir) = std::env::var("ICN_DATA_DIR") {
            config.data_dir = PathBuf::from(dir);
        }

        if let Ok(dir) = std::env::var("ICN_CERT_DIR") {
            config.cert_dir = PathBuf::from(dir);
        }

        if let Ok(dir) = std::env::var("ICN_LOG_DIR") {
            config.log_dir = PathBuf::from(dir);
        }

        if let Ok(level) = std::env::var("ICN_LOG_LEVEL") {
            config.log_level = level;
        }

        Ok(config)
    }

    pub fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.cert_dir)?;
        fs::create_dir_all(&self.log_dir)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_environment_config() {
        let env = Environment::default();
        assert_eq!(env.environment, "development");
        assert_eq!(env.log_level, "info");
        assert!(!env.debug);
        
        // Test validation
        assert!(env.validate().is_ok());
        
        // Test invalid environment
        let mut invalid_env = env.clone();
        invalid_env.environment = "invalid".to_string();
        assert!(invalid_env.validate().is_err());
        
        // Test invalid log level
        let mut invalid_log = env.clone();
        invalid_log.log_level = "invalid".to_string();
        assert!(invalid_log.validate().is_err());
    }
    
    #[test]
    fn test_config_file_operations() {
        // Create a temporary directory for the test
        let temp_dir = tempdir().expect("Failed to create temp directory");
        let config_path = temp_dir.path().join("config.toml");
        
        // Create a config
        let env = Environment {
            environment: "testing".to_string(),
            log_level: "debug".to_string(),
            debug: true,
        };
        
        // Save to file
        env.save_to_file(&config_path).expect("Failed to save config");
        
        // Load from file
        let loaded: Environment = Environment::from_file(&config_path)
            .expect("Failed to load config");
            
        assert_eq!(loaded.environment, "testing");
        assert_eq!(loaded.log_level, "debug");
        assert!(loaded.debug);
    }
}