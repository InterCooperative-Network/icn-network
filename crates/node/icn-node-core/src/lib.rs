//! Core node functionality for the Intercooperative Network
//!
//! This crate provides the fundamental components needed to run an ICN node,
//! including configuration, lifecycle management, and system integration.

use async_trait::async_trait;
use icn_common::{Error, Result};
use std::path::Path;
use std::sync::Arc;

mod config;
mod state;
mod systems;

pub use config::{NodeConfig, NetworkMode};
pub use state::{NodeState, StateManager};
pub use systems::{DidService, DidServiceConfig};

/// The Node trait defines the core functionality of an ICN node
#[async_trait]
pub trait Node {
    /// Initialize the node with the given configuration
    async fn initialize(config: NodeConfig) -> Result<Self> where Self: Sized;
    
    /// Start the node's services
    async fn start(&mut self) -> Result<()>;
    
    /// Stop the node's services
    async fn stop(&mut self) -> Result<()>;
    
    /// Check if the node is currently running
    fn is_running(&self) -> bool;
    
    /// Get the current state of the node
    fn state(&self) -> NodeState;
    
    /// Get the node's configuration
    fn config(&self) -> &NodeConfig;
}

/// Basic ICN node implementation
pub struct IcnNode {
    /// Node configuration
    config: NodeConfig,
    
    /// Node state manager
    state_manager: Arc<StateManager>,
    
    /// DID service
    did_service: Option<Arc<DidService>>,
}

impl IcnNode {
    /// Create a new node with the specified configuration
    pub async fn new(config: NodeConfig) -> Result<Self> {
        Self::initialize(config).await
    }
    
    /// Create a new node from a configuration file
    pub async fn from_config_file(path: impl AsRef<Path>) -> Result<Self> {
        let config = NodeConfig::from_file(path)?;
        Self::new(config).await
    }
    
    /// Get the DID service if enabled
    pub fn did_service(&self) -> Option<Arc<DidService>> {
        self.did_service.clone()
    }
}

#[async_trait]
impl Node for IcnNode {
    async fn initialize(config: NodeConfig) -> Result<Self> {
        // Validate configuration
        config.validate()?;
        
        // Create state manager
        let state_manager = Arc::new(StateManager::new());
        
        // Initialize DID service if enabled
        let did_service = if config.enable_identity {
            let did_config = DidServiceConfig {
                storage_options: icn_storage_system::StorageOptions {
                    base_dir: Path::new(&config.data_dir).join("did").to_path_buf(),
                    sync_writes: true,
                    compress: false,
                },
            };
            
            Some(Arc::new(DidService::new(did_config, state_manager.clone()).await?))
        } else {
            None
        };
        
        Ok(Self {
            config,
            state_manager,
            did_service,
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Err(Error::other("Node is already running"));
        }
        
        self.state_manager.transition(NodeState::Starting)?;
        
        // Start DID service if enabled
        if let Some(did_service) = &self.did_service {
            did_service.start().await?;
        }
        
        self.state_manager.transition(NodeState::Running)?;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        if !self.is_running() {
            return Err(Error::other("Node is not running"));
        }
        
        self.state_manager.transition(NodeState::Stopping)?;
        
        // Stop DID service if enabled
        if let Some(did_service) = &self.did_service {
            did_service.stop().await?;
        }
        
        self.state_manager.transition(NodeState::Stopped)?;
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        matches!(self.state_manager.current_state(), NodeState::Running)
    }
    
    fn state(&self) -> NodeState {
        self.state_manager.current_state()
    }
    
    fn config(&self) -> &NodeConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_node_lifecycle() {
        let temp_dir = tempdir().unwrap();
        
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_string_lossy().to_string();
        
        let mut node = IcnNode::new(config).await.unwrap();
        
        assert_eq!(node.state(), NodeState::Created);
        assert!(!node.is_running());
        
        node.start().await.unwrap();
        assert_eq!(node.state(), NodeState::Running);
        assert!(node.is_running());
        
        // Test DID service if enabled
        if let Some(did_service) = node.did_service() {
            let (doc, _) = did_service.create_did(CreateDidOptions::default()).await.unwrap();
            let resolved = did_service.resolve_did(&doc.id).await.unwrap();
            assert!(resolved.did_document.is_some());
        }
        
        node.stop().await.unwrap();
        assert_eq!(node.state(), NodeState::Stopped);
        assert!(!node.is_running());
    }
    
    #[tokio::test]
    async fn test_node_from_config() {
        let temp_dir = tempdir().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        
        let mut config = NodeConfig::default();
        config.data_dir = temp_dir.path().to_string_lossy().to_string();
        config.enable_identity = true;
        
        config.save_to_file(&config_path).unwrap();
        
        let node = IcnNode::from_config_file(&config_path).await.unwrap();
        assert!(node.did_service().is_some());
    }
}
