//! Identity-integrated storage system for ICN
//!
//! This module integrates the DID-based identity system with the
//! governance-controlled storage for robust authentication and authorization.

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;
use tracing::{debug, info, warn};

use crate::governance_storage::{GovernanceStorageService, StoragePolicyType, AccessPermission};

/// DID verification status
#[derive(Debug, Clone, PartialEq)]
pub enum DidVerificationStatus {
    /// DID verification succeeded
    Verified,
    /// DID verification failed
    Failed,
    /// DID not found
    NotFound,
    /// DID verification error
    Error(String),
}

/// DID document simplified for storage integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DidDocument {
    /// DID identifier (did:icn:...)
    pub id: String,
    /// Controller of this DID
    pub controller: Option<String>,
    /// Verification methods (keys)
    pub verification_method: Vec<VerificationMethod>,
    /// Authentication methods (references to verification methods)
    pub authentication: Vec<String>,
    /// Service endpoints
    pub service: Vec<ServiceEndpoint>,
}

/// Verification method (key) in a DID document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationMethod {
    /// ID of this verification method
    pub id: String,
    /// Type of verification method
    pub type_: String,
    /// Controller of this verification method
    pub controller: String,
    /// Public key material
    pub public_key_jwk: Option<serde_json::Value>,
    /// Public key as multibase
    pub public_key_multibase: Option<String>,
}

/// Service endpoint in a DID document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceEndpoint {
    /// ID of this service
    pub id: String,
    /// Type of service
    pub type_: String,
    /// Service endpoint URL
    pub service_endpoint: String,
}

/// Identity provider for DID resolution and verification
pub trait IdentityProvider {
    /// Resolve a DID to a DID document
    async fn resolve_did(&self, did: &str) -> Result<Option<DidDocument>>;
    
    /// Verify a signature using a DID
    async fn verify_signature(&self, did: &str, data: &[u8], signature: &[u8]) -> Result<DidVerificationStatus>;
    
    /// Get the member ID from a DID
    fn did_to_member_id(&self, did: &str) -> Result<String>;
}

/// Mock identity provider for testing
#[derive(Default)]
pub struct MockIdentityProvider {
    /// Mock DID documents
    did_documents: HashMap<String, DidDocument>,
}

impl MockIdentityProvider {
    /// Create a new mock identity provider
    pub fn new() -> Self {
        Self {
            did_documents: HashMap::new(),
        }
    }
    
    /// Add a mock DID document
    pub fn add_did_document(&mut self, did: String, document: DidDocument) {
        self.did_documents.insert(did, document);
    }
}

impl IdentityProvider for MockIdentityProvider {
    async fn resolve_did(&self, did: &str) -> Result<Option<DidDocument>> {
        Ok(self.did_documents.get(did).cloned())
    }
    
    async fn verify_signature(&self, did: &str, _data: &[u8], _signature: &[u8]) -> Result<DidVerificationStatus> {
        if self.did_documents.contains_key(did) {
            Ok(DidVerificationStatus::Verified)
        } else {
            Ok(DidVerificationStatus::NotFound)
        }
    }
    
    fn did_to_member_id(&self, did: &str) -> Result<String> {
        // Format: did:icn:federation:member
        let parts: Vec<&str> = did.split(':').collect();
        if parts.len() >= 4 && parts[0] == "did" && parts[1] == "icn" {
            Ok(parts[3].to_string())
        } else {
            Err(anyhow!("Invalid DID format: {}", did))
        }
    }
}

/// DID-based storage service that integrates identity with governance
pub struct IdentityStorageService<I: IdentityProvider> {
    /// Governance storage service
    governance_storage: GovernanceStorageService,
    /// Identity provider
    identity_provider: I,
    /// Authentication cache (DID -> timestamp)
    auth_cache: HashMap<String, u64>,
    /// Cache TTL in seconds
    cache_ttl: u64,
}

impl<I: IdentityProvider> IdentityStorageService<I> {
    /// Create a new identity storage service
    pub async fn new(
        federation: &str,
        data_path: impl Into<PathBuf>,
        identity_provider: I,
        cache_ttl: u64,
    ) -> Result<Self> {
        let governance_storage = GovernanceStorageService::new(federation, data_path).await?;
        
        Ok(Self {
            governance_storage,
            identity_provider,
            auth_cache: HashMap::new(),
            cache_ttl,
        })
    }
    
    /// Authenticate a user using their DID and signature
    pub async fn authenticate_did(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
    ) -> Result<DidVerificationStatus> {
        // Check the cache first
        let now = chrono::Utc::now().timestamp() as u64;
        if let Some(timestamp) = self.auth_cache.get(did) {
            if now - timestamp < self.cache_ttl {
                debug!("DID authentication cache hit for {}", did);
                return Ok(DidVerificationStatus::Verified);
            }
        }
        
        // Verify the signature
        let status = self.identity_provider.verify_signature(did, challenge, signature).await?;
        
        // If verified, update the cache
        if status == DidVerificationStatus::Verified {
            self.auth_cache.insert(did.to_string(), now);
        }
        
        Ok(status)
    }
    
    /// Store a file with DID authentication
    pub async fn store_file(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        file_path: impl AsRef<std::path::Path>,
        key: &str,
        encrypted: bool,
    ) -> Result<()> {
        // Authenticate the DID
        let status = self.authenticate_did(did, challenge, signature).await?;
        if status != DidVerificationStatus::Verified {
            return Err(anyhow!("DID authentication failed: {:?}", status));
        }
        
        // Convert DID to member ID
        let member_id = self.identity_provider.did_to_member_id(did)?;
        
        // Store the file with governance checks
        self.governance_storage.store_file(&member_id, file_path, key, encrypted).await
    }
    
    /// Retrieve a file with DID authentication
    pub async fn retrieve_file(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        key: &str,
        output_path: impl AsRef<std::path::Path>,
        version: Option<&str>,
    ) -> Result<()> {
        // Authenticate the DID
        let status = self.authenticate_did(did, challenge, signature).await?;
        if status != DidVerificationStatus::Verified {
            return Err(anyhow!("DID authentication failed: {:?}", status));
        }
        
        // Convert DID to member ID
        let member_id = self.identity_provider.did_to_member_id(did)?;
        
        // Retrieve the file with governance checks
        self.governance_storage.retrieve_file(&member_id, key, output_path, version).await
    }
    
    /// List files with DID authentication
    pub async fn list_files(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        prefix: Option<&str>,
    ) -> Result<Vec<crate::storage::VersionedFileMetadata>> {
        // Authenticate the DID
        let status = self.authenticate_did(did, challenge, signature).await?;
        if status != DidVerificationStatus::Verified {
            return Err(anyhow!("DID authentication failed: {:?}", status));
        }
        
        // Convert DID to member ID
        let member_id = self.identity_provider.did_to_member_id(did)?;
        
        // List files with governance checks
        self.governance_storage.list_files(&member_id, prefix).await
    }
    
    /// Map member DIDs to access permissions
    pub async fn update_did_access_mapping(&mut self, did_mappings: &[(String, String)]) -> Result<()> {
        // This would create or update a special policy that maps DIDs to member IDs
        let mappings: Vec<serde_json::Value> = did_mappings
            .iter()
            .map(|(did, member_id)| {
                serde_json::json!({
                    "did": did,
                    "member_id": member_id
                })
            })
            .collect();
        
        // Create the policy content
        let policy_content = serde_json::json!({
            "did_mappings": mappings
        });
        
        // Create a proposal for this policy
        // Note: In a real implementation, this would go through the normal governance process
        let proposal_id = self.governance_storage.propose_storage_policy(
            "system",
            "DID to Member ID Mappings",
            "Maps DIDs to member IDs for access control",
            StoragePolicyType::AccessControl,
            policy_content,
        ).await?;
        
        // For demo purposes, we'll directly apply the policy
        // In production, this would wait for approval through governance
        self.governance_storage.apply_approved_policy(&proposal_id).await?;
        
        Ok(())
    }
    
    /// Create a DID-based access control policy
    pub async fn create_did_access_policy(
        &mut self,
        did: &str,
        challenge: &[u8],
        signature: &[u8],
        access_permissions: &[AccessPermission],
    ) -> Result<String> {
        // Authenticate the DID
        let status = self.authenticate_did(did, challenge, signature).await?;
        if status != DidVerificationStatus::Verified {
            return Err(anyhow!("DID authentication failed: {:?}", status));
        }
        
        // Convert DID to member ID
        let proposer = self.identity_provider.did_to_member_id(did)?;
        
        // Create the policy content
        let policy_content = serde_json::json!(access_permissions);
        
        // Create a proposal for this policy
        let proposal_id = self.governance_storage.propose_storage_policy(
            &proposer,
            "DID-Based Access Control Policy",
            "Access control policy based on DID authentication",
            StoragePolicyType::AccessControl,
            policy_content,
        ).await?;
        
        Ok(proposal_id)
    }
    
    /// Forward other methods to the underlying governance storage service
    pub fn get_governance_storage(&self) -> &GovernanceStorageService {
        &self.governance_storage
    }
    
    /// Get a mutable reference to the underlying governance storage service
    pub fn get_governance_storage_mut(&mut self) -> &mut GovernanceStorageService {
        &mut self.governance_storage
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_did_authentication() -> Result<()> {
        // Create a test DID document
        let did = "did:icn:test:alice";
        let document = DidDocument {
            id: did.to_string(),
            controller: None,
            verification_method: vec![
                VerificationMethod {
                    id: format!("{}#keys-1", did),
                    type_: "Ed25519VerificationKey2020".to_string(),
                    controller: did.to_string(),
                    public_key_jwk: None,
                    public_key_multibase: Some("z6MkhaXgBZDvotDkL5257faiztiGiC2QtKLGpbnnEGta2doK".to_string()),
                }
            ],
            authentication: vec![format!("{}#keys-1", did)],
            service: vec![],
        };
        
        // Create a mock identity provider
        let mut provider = MockIdentityProvider::new();
        provider.add_did_document(did.to_string(), document);
        
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        
        // Create an identity storage service
        let mut service = IdentityStorageService::new(
            "test",
            temp_dir.path(),
            provider,
            3600, // 1 hour cache TTL
        ).await?;
        
        // Test authentication
        let challenge = b"test challenge";
        let signature = b"test signature"; // In a real test, this would be a valid signature
        
        let status = service.authenticate_did(did, challenge, signature).await?;
        assert_eq!(status, DidVerificationStatus::Verified);
        
        // Test mapping DID to member ID
        let member_id = service.identity_provider.did_to_member_id(did)?;
        assert_eq!(member_id, "alice");
        
        Ok(())
    }
} 