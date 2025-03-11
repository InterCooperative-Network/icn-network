//! Node system components

pub mod capabilities;
pub mod did_service;

pub use capabilities::{Capability, FederationCapability};
pub use did_service::DidService;

use std::sync::Arc;
use crate::state::StateManager;
use crate::config::NodeConfig;
use icn_common::Result;

/// Core node systems manager
pub struct SystemsManager {
    state: Arc<StateManager>,
    did_service: Option<Arc<DidService>>,
}

impl SystemsManager {
    /// Create a new systems manager
    pub fn new(state: Arc<StateManager>) -> Self {
        Self {
            state,
            did_service: None,
        }
    }
    
    /// Initialize all systems based on node configuration
    pub async fn initialize(&mut self, config: &NodeConfig) -> Result<()> {
        // Initialize DID service if enabled
        if config.capabilities.storage_enabled {
            let did_service = DidService::from_config(config, self.state.clone()).await?;
            self.did_service = Some(Arc::new(did_service));
        }
        
        Ok(())
    }
    
    /// Start all initialized systems
    pub async fn start(&self) -> Result<()> {
        if let Some(did_service) = &self.did_service {
            did_service.start().await?;
        }
        Ok(())
    }
    
    /// Stop all systems
    pub async fn stop(&self) -> Result<()> {
        if let Some(did_service) = &self.did_service {
            did_service.stop().await?;
        }
        Ok(())
    }
    
    /// Get reference to DID service if enabled
    pub fn did_service(&self) -> Option<Arc<DidService>> {
        self.did_service.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use icn_storage_system::StorageOptions;
    
    #[tokio::test]
    async fn test_systems_lifecycle() {
        let state = Arc::new(StateManager::new());
        let mut manager = SystemsManager::new(state);
        
        let temp_dir = tempdir().unwrap();
        let config = NodeConfig {
            node_id: "test-node".to_string(),
            federation_id: "test-federation".to_string(),
            federation_endpoints: vec!["http://test.federation".to_string()],
            storage: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            capabilities: crate::config::CapabilitiesConfig {
                storage_enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        
        // Initialize systems
        manager.initialize(&config).await.unwrap();
        assert!(manager.did_service().is_some());
        
        // Start systems
        manager.start().await.unwrap();
        
        // Stop systems
        manager.stop().await.unwrap();
    }
}