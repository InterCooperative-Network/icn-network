//! Command line-based configuration for ICN
//!
//! This module provides a configuration provider that reads configuration from
//! command line arguments.

use std::path::PathBuf;
use std::sync::Arc;
use clap::{Arg, ArgAction, ArgMatches, Command};
use tokio::sync::RwLock;
use super::{NodeConfig, NetworkConfig, StorageConfig, IdentityConfig, ConfigProvider, ConfigResult};

/// A command line-based configuration provider
pub struct CommandLineConfigProvider {
    /// Base configuration to use as fallback
    base_config: NodeConfig,
    /// Cached configuration
    config: Arc<RwLock<Option<NodeConfig>>>,
    /// Command line arguments
    arg_matches: Arc<ArgMatches>,
}

impl CommandLineConfigProvider {
    /// Create a new command line-based configuration provider
    pub fn new() -> Self {
        Self::with_base_config(NodeConfig::default())
    }
    
    /// Create a new provider with a specific base configuration
    pub fn with_base_config(base_config: NodeConfig) -> Self {
        Self {
            base_config,
            config: Arc::new(RwLock::new(None)),
            arg_matches: Arc::new(Self::parse_args(&base_config)),
        }
    }
    
    /// Create a new provider with specific command line arguments
    pub fn with_args(base_config: NodeConfig, args: Vec<String>) -> Self {
        Self {
            base_config,
            config: Arc::new(RwLock::new(None)),
            arg_matches: Arc::new(Self::create_app(&base_config).get_matches_from(args)),
        }
    }
    
    /// Parse command line arguments
    fn parse_args(base_config: &NodeConfig) -> ArgMatches {
        Self::create_app(base_config).get_matches()
    }
    
    /// Create the command line parser
    fn create_app(base_config: &NodeConfig) -> Command {
        Command::new("icn-node")
            .version(env!("CARGO_PKG_VERSION"))
            .about("InterCooperative Network Node")
            // Network configuration
            .arg(
                Arg::new("host")
                    .long("host")
                    .value_name("HOST")
                    .help("Host to bind to")
                    .default_value(&base_config.network.host)
            )
            .arg(
                Arg::new("port")
                    .long("port")
                    .short('p')
                    .value_name("PORT")
                    .help("Port to bind to")
                    .default_value(&base_config.network.port.to_string())
            )
            .arg(
                Arg::new("bootstrap")
                    .long("bootstrap")
                    .value_name("NODES")
                    .help("Comma-separated list of bootstrap nodes")
                    .default_value("")
            )
            // Storage configuration
            .arg(
                Arg::new("storage-path")
                    .long("storage")
                    .value_name("PATH")
                    .help("Path to storage directory")
                    .default_value(base_config.storage.path.to_str().unwrap_or("data"))
            )
            .arg(
                Arg::new("sync-writes")
                    .long("sync-writes")
                    .help("Sync writes immediately")
                    .action(ArgAction::SetTrue)
            )
            .arg(
                Arg::new("no-sync-writes")
                    .long("no-sync-writes")
                    .help("Don't sync writes immediately")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("sync-writes")
            )
            .arg(
                Arg::new("use-cache")
                    .long("cache")
                    .help("Use storage cache")
                    .action(ArgAction::SetTrue)
            )
            .arg(
                Arg::new("no-cache")
                    .long("no-cache")
                    .help("Don't use storage cache")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("use-cache")
            )
            // Identity configuration
            .arg(
                Arg::new("key-file")
                    .long("key-file")
                    .value_name("FILE")
                    .help("Path to identity key file")
                    .default_value(base_config.identity.key_file.to_str().unwrap_or("identity.key"))
            )
            .arg(
                Arg::new("generate-identity")
                    .long("generate-identity")
                    .help("Generate a new identity if one doesn't exist")
                    .action(ArgAction::SetTrue)
            )
            .arg(
                Arg::new("no-generate-identity")
                    .long("no-generate-identity")
                    .help("Don't generate a new identity if one doesn't exist")
                    .action(ArgAction::SetTrue)
                    .conflicts_with("generate-identity")
            )
            .arg(
                Arg::new("name")
                    .long("name")
                    .value_name("NAME")
                    .help("Friendly name for this node")
                    .default_value(&base_config.identity.friendly_name)
            )
            // Other configuration
            .arg(
                Arg::new("environment")
                    .long("env")
                    .value_name("ENV")
                    .help("Environment (development, production)")
                    .default_value(&base_config.environment)
            )
            .arg(
                Arg::new("log-level")
                    .long("log-level")
                    .value_name("LEVEL")
                    .help("Log level (trace, debug, info, warn, error)")
                    .default_value(&base_config.log_level)
            )
            .arg(
                Arg::new("config-file")
                    .long("config")
                    .short('c')
                    .value_name("FILE")
                    .help("Path to configuration file")
            )
    }
}

#[async_trait::async_trait]
impl ConfigProvider for CommandLineConfigProvider {
    async fn get_config(&self) -> ConfigResult<NodeConfig> {
        // Try to get from cache first
        {
            let config = self.config.read().await;
            if let Some(config) = config.as_ref() {
                return Ok(config.clone());
            }
        }
        
        // Start with the base configuration
        let mut config = self.base_config.clone();
        let matches = &self.arg_matches;
        
        // Network configuration
        if let Some(host) = matches.get_one::<String>("host") {
            config.network.host = host.clone();
        }
        
        if let Some(port) = matches.get_one::<String>("port") {
            if let Ok(port) = port.parse::<u16>() {
                config.network.port = port;
            }
        }
        
        if let Some(bootstrap) = matches.get_one::<String>("bootstrap") {
            if !bootstrap.is_empty() {
                config.network.bootstrap_nodes = bootstrap
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
        
        // Storage configuration
        if let Some(path) = matches.get_one::<String>("storage-path") {
            config.storage.path = PathBuf::from(path);
        }
        
        if matches.get_flag("sync-writes") {
            config.storage.sync_writes = true;
        } else if matches.get_flag("no-sync-writes") {
            config.storage.sync_writes = false;
        }
        
        if matches.get_flag("use-cache") {
            config.storage.use_cache = true;
        } else if matches.get_flag("no-cache") {
            config.storage.use_cache = false;
        }
        
        // Identity configuration
        if let Some(key_file) = matches.get_one::<String>("key-file") {
            config.identity.key_file = PathBuf::from(key_file);
        }
        
        if matches.get_flag("generate-identity") {
            config.identity.generate_if_missing = true;
        } else if matches.get_flag("no-generate-identity") {
            config.identity.generate_if_missing = false;
        }
        
        if let Some(name) = matches.get_one::<String>("name") {
            config.identity.friendly_name = name.clone();
        }
        
        // Other configuration
        if let Some(env) = matches.get_one::<String>("environment") {
            config.environment = env.clone();
        }
        
        if let Some(log_level) = matches.get_one::<String>("log-level") {
            config.log_level = log_level.clone();
        }
        
        // Update cache
        {
            let mut cache = self.config.write().await;
            *cache = Some(config.clone());
        }
        
        Ok(config)
    }
    
    async fn set_config(&self, config: NodeConfig) -> ConfigResult<()> {
        // We can only update the cache, as command line args can't be changed after the program starts
        let mut cache = self.config.write().await;
        *cache = Some(config);
        Ok(())
    }
} 