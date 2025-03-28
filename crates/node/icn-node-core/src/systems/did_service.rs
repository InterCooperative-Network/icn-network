//! DID service implementation for ICN nodes
//! 
//! This module implements the DID service component that provides
//! identity management capabilities to ICN nodes.

use async_trait::async_trait;
use icn_common::{Error, Result};
use icn_did::{
    DidManager, DidManagerConfig, CreateDidOptions, DidDocument,
    resolver::{DidResolver, ResolutionResult},
    verification::{AuthenticationChallenge, AuthenticationResponse},
};
use icn_storage_system::StorageOptions;
use std::sync::Arc;
use tokio::sync::RwLock;
use crate::state::{ComponentState, StateManager};
use icn_crypto::Signature;
use super::capabilities::{Capability, FederationCapability, FederationRequest, FederationResponse};

/// DID service configuration
#[derive(Debug, Clone)]
pub struct DidServiceConfig {
    /// Storage options for the DID system
    pub storage_options: StorageOptions,
    
    /// Federation ID for this node
    pub federation_id: String,
    
    /// Federation endpoints
    pub federation_endpoints: Vec<String>,
}

impl Default for DidServiceConfig {
    fn default() -> Self {
        Self {
            storage_options: StorageOptions::default(),
            federation_id: "local".to_string(),
            federation_endpoints: Vec::new(),
        }
    }
}

/// DID service component
pub struct DidService {
    /// The DID manager
    manager: Arc<DidManager>,
    
    /// State manager
    state_manager: Arc<StateManager>,
    
    /// Active authentication challenges
    challenges: Arc<RwLock<Vec<AuthenticationChallenge>>>,
    
    /// Running state
    running: bool,
}

impl DidService {
    /// Create a new DID service
    pub async fn new(
        config: DidServiceConfig,
        state_manager: Arc<StateManager>,
    ) -> Result<Self> {
        // Create DID manager config
        let manager_config = DidManagerConfig {
            storage_options: config.storage_options,
            federation_id: config.federation_id,
            federation_endpoints: config.federation_endpoints,
            ..Default::default()
        };
        
        // Initialize DID manager
        let manager = DidManager::new(manager_config).await?;
        
        // Register with state manager
        state_manager.register_component("did_service", ComponentState::default())?;
        
        Ok(Self {
            manager: Arc::new(manager),
            state_manager,
            challenges: Arc::new(RwLock::new(Vec::new())),
            running: false,
        })
    }

    /// Create a new DID service from node config
    pub async fn from_config(
        config: &crate::config::NodeConfig,
        state_manager: Arc<StateManager>,
    ) -> Result<Self> {
        let service_config = DidServiceConfig {
            storage_options: config.storage.clone(),
            federation_id: config.federation_id.clone(),
            federation_endpoints: config.federation_endpoints.clone(),
        };

        Self::new(service_config, state_manager).await
    }
    
    /// Start the DID service
    pub async fn start(&self) -> Result<()> {
        self.state_manager.update_component("did_service", "starting")?;
        
        // Start cleanup task for expired challenges
        let challenges = self.challenges.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
                let mut challenges = challenges.write().await;
                challenges.retain(|c| !c.is_expired().unwrap_or(true));
            }
        });
        
        self.state_manager.update_component("did_service", "running")?;
        Ok(())
    }
    
    /// Stop the DID service
    pub async fn stop(&self) -> Result<()> {
        self.state_manager.update_component("did_service", "stopping")?;
        self.state_manager.update_component("did_service", "stopped")?;
        Ok(())
    }
    
    /// Create a new DID
    pub async fn create_did(&self, options: CreateDidOptions) -> Result<(DidDocument, String)> {
        let (document, key_pair) = self.manager.create_did(options).await?;
        
        // Extract public key for return
        let public_key = key_pair.public_key().to_base58();
        
        Ok((document, public_key))
    }
    
    /// Create a new federated DID
    pub async fn create_federated_did(
        &self,
        options: CreateDidOptions,
        federation_id: Option<String>,
    ) -> Result<(DidDocument, String)> {
        let (document, key_pair) = self.manager
            .create_federated_did(options, federation_id)
            .await?;
            
        let public_key = key_pair.public_key().to_base58();
        Ok((document, public_key))
    }
    
    /// Resolve a DID
    pub async fn resolve_did(&self, did: &str) -> Result<ResolutionResult> {
        self.manager.resolve(did).await
    }
    
    /// Update a DID document
    pub async fn update_did(&self, did: &str, document: DidDocument) -> Result<()> {
        self.manager.update_did(did, document).await
    }
    
    /// Deactivate a DID
    pub async fn deactivate_did(&self, did: &str) -> Result<()> {
        self.manager.deactivate_did(did).await
    }
    
    /// List all DIDs
    pub async fn list_dids(&self) -> Result<Vec<String>> {
        self.manager.list_dids().await
    }
    
    /// Create an authentication challenge
    pub async fn create_authentication_challenge(
        &self,
        did: &str,
        verification_method: Option<&str>,
    ) -> Result<AuthenticationChallenge> {
        let challenge = self.manager
            .create_authentication_challenge(did, verification_method, None)
            .await?;
            
        // Store challenge
        self.challenges.write().await.push(challenge.clone());
        
        Ok(challenge)
    }
    
    /// Verify an authentication response
    pub async fn verify_authentication(
        &self,
        response: &AuthenticationResponse,
    ) -> Result<bool> {
        // Verify challenge exists
        let challenges = self.challenges.read().await;
        if !challenges.iter().any(|c| c.nonce == response.challenge.nonce) {
            return Ok(false);
        }
        
        // Verify response
        let result = self.manager.verify_authentication(response).await?;
        
        // Remove challenge if valid
        if result {
            let mut challenges = self.challenges.write().await;
            challenges.retain(|c| c.nonce != response.challenge.nonce);
        }
        
        Ok(result)
    }
    
    /// Verify a signature
    pub async fn verify_signature(
        &self,
        did: &str,
        method_id: &str,
        message: &[u8],
        signature: &icn_crypto::Signature,
    ) -> Result<bool> {
        self.manager.verify_signature(did, method_id, message, signature).await
    }
    
    /// Get the DID manager
    pub fn manager(&self) -> Arc<DidManager> {
        self.manager.clone()
    }
}

#[async_trait]
impl Capability for DidService {
    async fn start(&self) -> Result<()> {
        self.state_manager.register_component("did_service")?;
        self.state_manager.update_component("did_service", "running")?;
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        self.state_manager.update_component("did_service", "stopped")?;
        Ok(())
    }

    fn is_running(&self) -> bool {
        self.running
    }
}

#[async_trait]
impl FederationCapability for DidService {
    fn federation_id(&self) -> &str {
        &self.manager.federation_id()
    }

    fn federation_endpoints(&self) -> &[String] {
        self.manager.federation_endpoints()
    }

    async fn handle_federation_request(&self, request: FederationRequest) -> Result<FederationResponse> {
        match request {
            FederationRequest::ResolveDid { did, federation_id } => {
                let result = self.manager
                    .handle_federation_resolution(&did, &federation_id)
                    .await?;
                
                Ok(FederationResponse::DidResolution {
                    document: result.did_document,
                    error: result.resolution_metadata.error,
                })
            }
            FederationRequest::VerifyDid { did, challenge, signature } => {
                let is_valid = self.manager
                    .verify_signature(&did, "#key-1", &challenge, &signature)
                    .await?;
                    
                Ok(FederationResponse::Verification {
                    is_valid,
                    error: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use icn_did::Service;
    use crate::config::{NodeConfig, NetworkConfig, CapabilitiesConfig};

    #[tokio::test]
    async fn test_did_service_lifecycle() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = DidServiceConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        // Create service
        let service = DidService::new(config, state_manager.clone()).await.unwrap();
        
        // Start service
        service.start().await.unwrap();
        
        // Check component state
        let component = state_manager.get_component("did_service").unwrap();
        assert_eq!(component.state, "running");
        
        // Create a DID
        let (document, _) = service.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Resolve the DID
        let resolution = service.resolve_did(&document.id).await.unwrap();
        assert!(resolution.did_document.is_some());
        assert_eq!(resolution.did_document.unwrap().id, document.id);
        
        // Stop service
        service.stop().await.unwrap();
        
        // Check component state
        let component = state_manager.get_component("did_service").unwrap();
        assert_eq!(component.state, "stopped");
    }

    #[tokio::test]
    async fn test_did_service_operations() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = DidServiceConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        let service = DidService::new(config, state_manager).await.unwrap();
        service.start().await.unwrap();
        
        // Test creating a DID with options
        let options = CreateDidOptions {
            additional_services: vec![
                Service {
                    id: "service-1".to_string(),
                    type_: "MessagingService".to_string(),
                    service_endpoint: "https://messaging.example.com".to_string(),
                }
            ],
            ..Default::default()
        };
        
        let (doc, public_key) = service.create_did(options).await.unwrap();
        assert!(!public_key.is_empty());
        assert_eq!(doc.service.len(), 1);
        
        // Test updating the DID document
        let mut updated_doc = doc.clone();
        updated_doc.service.push(Service {
            id: format!("{}#service-2", doc.id),
            type_: "StorageService".to_string(),
            service_endpoint: "https://storage.example.com".to_string(),
        });
        
        service.update_did(&doc.id, updated_doc.clone()).await.unwrap();
        
        // Verify update
        let resolution = service.resolve_did(&doc.id).await.unwrap();
        let resolved_doc = resolution.did_document.unwrap();
        assert_eq!(resolved_doc.service.len(), 2);
        
        // Test listing DIDs
        let dids = service.list_dids().await.unwrap();
        assert_eq!(dids.len(), 1);
        assert!(dids.contains(&doc.id));
        
        // Test deactivating DID
        service.deactivate_did(&doc.id).await.unwrap();
        
        // Verify deactivation
        let resolution = service.resolve_did(&doc.id).await.unwrap();
        assert!(resolution.document_metadata.deactivated.unwrap());
    }
    
    #[tokio::test]
    async fn test_error_handling() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = DidServiceConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        let service = DidService::new(config, state_manager).await.unwrap();
        
        // Test resolving non-existent DID
        let result = service.resolve_did("did:icn:nonexistent").await.unwrap();
        assert!(result.did_document.is_none());
        assert_eq!(result.resolution_metadata.error.unwrap(), "notFound");
        
        // Test updating non-existent DID
        let doc = DidDocument::new("test123").unwrap();
        let result = service.update_did("did:icn:nonexistent", doc);
        assert!(result.await.is_err());
        
        // Test deactivating non-existent DID
        let result = service.deactivate_did("did:icn:nonexistent");
        assert!(result.await.is_err());
    }
    
    #[tokio::test]
    async fn test_concurrent_operations() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = DidServiceConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        let service = Arc::new(DidService::new(config, state_manager).await.unwrap());
        service.start().await.unwrap();
        
        // Create multiple DIDs concurrently
        let mut handles = vec![];
        for _ in 0..5 {
            let service_clone = service.clone();
            handles.push(tokio::spawn(async move {
                service_clone.create_did(CreateDidOptions::default()).await.unwrap()
            }));
        }
        
        // Wait for all operations to complete
        let results = futures::future::join_all(handles).await;
        let dids: Vec<_> = results.into_iter()
            .map(|r| r.unwrap().0.id)
            .collect();
        
        // Verify all DIDs were created
        let listed_dids = service.list_dids().await.unwrap();
        assert_eq!(listed_dids.len(), 5);
        
        for did in dids {
            assert!(listed_dids.contains(&did));
        }
    }

    #[tokio::test]
    async fn test_authentication_flow() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = DidServiceConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        let service = DidService::new(config, state_manager).await.unwrap();
        service.start().await.unwrap();
        
        // Create a DID with key pair
        let (doc, key_pair) = service
            .create_did(CreateDidOptions::default())
            .await
            .unwrap();
            
        // Create authentication challenge
        let challenge = service
            .create_authentication_challenge(&doc.id, None)
            .await
            .unwrap();
            
        assert_eq!(challenge.did, doc.id);
        
        // Sign challenge
        let signature = key_pair.sign(&challenge.get_message()).unwrap();
        
        // Create and verify response
        let response = AuthenticationResponse {
            challenge: challenge.clone(),
            signature,
        };
        
        let result = service.verify_authentication(&response).await.unwrap();
        assert!(result);
        
        // Verify challenge was removed
        assert!(service.challenges.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_signature_verification() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = DidServiceConfig {
            storage_options: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            ..Default::default()
        };
        
        let service = DidService::new(config, state_manager).await.unwrap();
        service.start().await.unwrap();
        
        // Create a DID with key pair
        let (doc, key_pair) = service
            .create_did(CreateDidOptions::default())
            .await
            .unwrap();
            
        // Create and sign a message
        let message = b"test message";
        let signature = key_pair.sign(message).unwrap();
        
        // Verify signature
        let result = service
            .verify_signature(&doc.id, "#key-1", message, &signature)
            .await
            .unwrap();
            
        assert!(result);
        
        // Test with invalid signature
        let invalid_sig = icn_crypto::Signature::new(vec![0; 64]);
        let result = service
            .verify_signature(&doc.id, "#key-1", message, &invalid_sig)
            .await
            .unwrap();
            
        assert!(!result);
    }

    #[tokio::test]
    async fn test_service_from_config() {
        let temp_dir = tempdir().unwrap();
        let state_manager = Arc::new(StateManager::new());
        
        let config = NodeConfig {
            node_id: "test-node".to_string(),
            federation_id: "test-federation".to_string(),
            federation_endpoints: vec![
                "http://federation1.test".to_string(),
                "http://federation2.test".to_string(),
            ],
            storage: StorageOptions {
                base_dir: temp_dir.path().to_path_buf(),
                sync_writes: true,
                compress: false,
            },
            capabilities: CapabilitiesConfig::default(),
            network: NetworkConfig::default(),
        };

        let service = DidService::from_config(&config, state_manager).await.unwrap();
        
        // Verify service initialized with correct federation config 
        let did = "did:icn:test-federation:123";
        let doc = DidDocument::new("test-federation:123").unwrap();
        
        service.create_did(CreateDidOptions::default()).await.unwrap();
        
        // Test federation resolution
        let result = service.resolve_did(did).await.unwrap();
        assert_eq!(result.resolution_metadata.source_federation.unwrap(), "test-federation");
    }
}