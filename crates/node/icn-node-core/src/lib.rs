//! Core node functionality for the Intercooperative Network
//!
//! This crate provides the fundamental components needed to run an ICN node,
//! including configuration, lifecycle management, and system integration.

use async_trait::async_trait;
use icn_common::{Error, Result};
use std::path::Path;

mod config;
mod state;
mod systems;

pub use config::{NodeConfig, NetworkMode};
pub use state::NodeState;

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
    config: NodeConfig,
    state: NodeState,
}

#[async_trait]
impl Node for IcnNode {
    async fn initialize(config: NodeConfig) -> Result<Self> {
        // Validate configuration
        config.validate()?;
        
        Ok(Self {
            config,
            state: NodeState::Initialized,
        })
    }
    
    async fn start(&mut self) -> Result<()> {
        if self.is_running() {
            return Err(Error::other("Node is already running"));
        }
        
        // Start systems
        // TODO: Initialize and start individual systems
        
        self.state = NodeState::Running;
        Ok(())
    }
    
    async fn stop(&mut self) -> Result<()> {
        if !self.is_running() {
            return Err(Error::other("Node is not running"));
        }
        
        // Stop systems
        // TODO: Stop individual systems
        
        self.state = NodeState::Stopped;
        Ok(())
    }
    
    fn is_running(&self) -> bool {
        matches!(self.state, NodeState::Running)
    }
    
    fn state(&self) -> NodeState {
        self.state
    }
    
    fn config(&self) -> &NodeConfig {
        &self.config
    }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_node_lifecycle() {
        let config = NodeConfig::default();
        let mut node = IcnNode::new(config).await.unwrap();
        
        assert_eq!(node.state(), NodeState::Initialized);
        assert!(!node.is_running());
        
        node.start().await.unwrap();
        assert_eq!(node.state(), NodeState::Running);
        assert!(node.is_running());
        
        node.stop().await.unwrap();
        assert_eq!(node.state(), NodeState::Stopped);
        assert!(!node.is_running());
    }
}
