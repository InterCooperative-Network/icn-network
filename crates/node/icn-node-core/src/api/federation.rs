use async_trait::async_trait;
use icn_common::{Error, Result};
use std::sync::Arc;
use crate::systems::{DidService, FederationCapability, FederationRequest, FederationResponse};

/// Federation API handler
pub struct FederationApi {
    did_service: Arc<DidService>,
}

impl FederationApi {
    /// Create a new federation API handler
    pub fn new(did_service: Arc<DidService>) -> Self {
        Self {
            did_service,
        }
    }
    
    /// Get federation information
    pub async fn get_federation_info(&self) -> Result<FederationInfo> {
        Ok(FederationInfo {
            federation_id: self.did_service.federation_id().to_string(),
            endpoints: self.did_service.federation_endpoints().to_vec(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }
    
    /// Handle DID resolution from another federation
    pub async fn resolve_did(&self, did: &str, federation_id: &str) -> Result<FederationResponse> {
        let request = FederationRequest::ResolveDid {
            did: did.to_string(),
            federation_id: federation_id.to_string(),
        };
        
        self.did_service.handle_federation_request(request).await
    }
    
    /// Handle DID verification from another federation
    pub async fn verify_did_signature(
        &self, 
        did: &str, 
        challenge: &[u8], 
        signature: &[u8]
    ) -> Result<FederationResponse> {
        let request = FederationRequest::VerifyDid {
            did: did.to_string(),
            challenge: challenge.to_vec(),
            signature: signature.to_vec(),
        };
        
        self.did_service.handle_federation_request(request).await
    }
    
    /// Lookup federation information for another federation
    pub async fn lookup_federation(&self, federation_id: &str) -> Result<Option<FederationInfo>> {
        // Call federation directory to lookup federation info
        // For now, just return federation configuration if it's the local federation
        if federation_id == self.did_service.federation_id() {
            return Ok(Some(FederationInfo {
                federation_id: self.did_service.federation_id().to_string(),
                endpoints: self.did_service.federation_endpoints().to_vec(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            }));
        }
        
        // In a real implementation, we would query a federation directory or use other discovery methods
        Ok(None)
    }
}

/// Federation information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FederationInfo {
    /// Federation ID
    pub federation_id: String,
    
    /// Federation API endpoints
    pub endpoints: Vec<String>,
    
    /// Federation software version
    pub version: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use crate::state::StateManager;
    use crate::config::NodeConfig;
    use icn_storage_system::StorageOptions;
    
    #[tokio::test]
    async fn test_federation_api() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        // Create node config
        let config = NodeConfig {
            node_id: "test-node".to_string(),
            federation_id: "test-federation".to_string(),
            federation_endpoints: vec!["http://federation.test/api".to_string()],
            storage: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        // Create DID service
        let did_service = Arc::new(DidService::from_config(&config, state_manager).await.unwrap());
        
        // Create API handler
        let api = FederationApi::new(did_service);
        
        // Test federation info
        let info = api.get_federation_info().await.unwrap();
        assert_eq!(info.federation_id, "test-federation");
        assert_eq!(info.endpoints.len(), 1);
        assert_eq!(info.endpoints[0], "http://federation.test/api");
        
        // Test federation lookup
        let lookup = api.lookup_federation("test-federation").await.unwrap();
        assert!(lookup.is_some());
        assert_eq!(lookup.unwrap().federation_id, "test-federation");
        
        let lookup = api.lookup_federation("unknown-federation").await.unwrap();
        assert!(lookup.is_none());
        
        // Test DID resolution
        let did = "did:icn:test-federation:123";
        let response = api.resolve_did(did, "test-federation").await.unwrap();
        match response {
            FederationResponse::DidResolution { document, error } => {
                // Document doesn't exist but resolution was successful
                assert!(document.is_none());
                assert!(error.is_some());
            }
            _ => panic!("Unexpected response type"),
        }
    }
}