//! Common configuration utilities
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::error::{Error, Result};

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

/// Load environment variables into a configuration
pub fn load_from_env<T: Configuration>(prefix: &str) -> T {
    use std::env;
    let mut config = T::default();
    
    // Implementation would look for environment variables with the given prefix
    // and update the config accordingly. This is a placeholder for now.
    
    config
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