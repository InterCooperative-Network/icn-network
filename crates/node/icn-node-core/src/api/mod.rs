pub mod federation;

use icn_common::{Error, Result};
use std::sync::Arc;
use crate::systems::SystemsManager;
use federation::FederationApi;

/// API server for the node
pub struct ApiServer {
    systems: Arc<SystemsManager>,
    federation_api: Option<FederationApi>,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(systems: Arc<SystemsManager>) -> Self {
        let mut server = Self {
            systems,
            federation_api: None,
        };
        
        server.initialize();
        server
    }
    
    /// Initialize API handlers
    fn initialize(&mut self) {
        // Initialize federation API if DID service is available
        if let Some(did_service) = self.systems.did_service() {
            self.federation_api = Some(FederationApi::new(did_service));
        }
    }
    
    /// Get federation API handler
    pub fn federation_api(&self) -> Option<&FederationApi> {
        self.federation_api.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::state::StateManager;
    use crate::config::NodeConfig;
    use icn_storage_system::StorageOptions;
    
    #[tokio::test]
    async fn test_api_server_init() {
        // Create state manager and systems manager
        let state = Arc::new(StateManager::new());
        let mut systems = SystemsManager::new(state);
        
        // Create temp directory for storage
        let temp_dir = tempdir().unwrap();
        let config = NodeConfig {
            node_id: "test-node".to_string(),
            federation_id: "test-federation".to_string(),
            federation_endpoints: vec!["http://federation.test/api".to_string()],
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
        systems.initialize(&config).await.unwrap();
        systems.start().await.unwrap();
        
        // Create API server
        let api_server = ApiServer::new(Arc::new(systems));
        
        // Check federation API is available
        assert!(api_server.federation_api().is_some());
    }
}