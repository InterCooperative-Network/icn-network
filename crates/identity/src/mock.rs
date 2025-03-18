//! Mock identity provider for testing
//!
//! This module provides a mock implementation of the IdentityProvider
//! trait for use in tests.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use icn_core::{
    crypto::{identity::NodeId, Signature},
    utils::timestamp_secs,
};

use crate::{
    Identity, IdentityProvider, IdentityResult, IdentityError,
};

/// A mock implementation of the IdentityProvider for testing
pub struct MockIdentityProvider {
    /// The current identity
    current_identity: Arc<RwLock<Option<Identity>>>,
    /// Known identities
    identities: Arc<RwLock<HashMap<String, Identity>>>,
    /// Flag to control verification responses
    always_verify: Arc<RwLock<bool>>,
}

impl MockIdentityProvider {
    /// Create a new MockIdentityProvider
    pub fn new() -> Self {
        // Create a default identity
        let id = "mock-identity-1".to_string();
        let identity = Identity {
            id: id.clone(),
            name: "Mock User".to_string(),
            public_key: vec![0, 1, 2, 3, 4],
            metadata: HashMap::new(),
            created_at: timestamp_secs(),
            updated_at: timestamp_secs(),
        };
        
        let mut identities = HashMap::new();
        identities.insert(id, identity.clone());
        
        Self {
            current_identity: Arc::new(RwLock::new(Some(identity))),
            identities: Arc::new(RwLock::new(identities)),
            always_verify: Arc::new(RwLock::new(true)),
        }
    }
    
    /// Set whether verification should always succeed
    pub fn set_always_verify(&self, value: bool) {
        let mut always_verify = self.always_verify.write().unwrap();
        *always_verify = value;
    }
    
    /// Add an identity to the known identities
    pub fn add_identity(&self, identity: Identity) {
        let mut identities = self.identities.write().unwrap();
        identities.insert(identity.id.clone(), identity);
    }
    
    /// Set the current identity
    pub fn set_current_identity(&self, identity: Identity) {
        // Add to known identities
        self.add_identity(identity.clone());
        
        // Set as current
        let mut current = self.current_identity.write().unwrap();
        *current = Some(identity);
    }
}

#[async_trait]
impl IdentityProvider for MockIdentityProvider {
    /// Get the current identity
    async fn get_identity(&self) -> IdentityResult<Identity> {
        let current = self.current_identity.read().unwrap();
        current.clone().ok_or(IdentityError::NoIdentity)
    }
    
    /// Create a new identity
    async fn create_identity(&self, name: &str, metadata: HashMap<String, String>) -> IdentityResult<Identity> {
        let id = format!("mock-identity-{}", timestamp_secs());
        let identity = Identity {
            id: id.clone(),
            name: name.to_string(),
            public_key: vec![0, 1, 2, 3, 4],
            metadata,
            created_at: timestamp_secs(),
            updated_at: timestamp_secs(),
        };
        
        // Add to known identities
        self.add_identity(identity.clone());
        
        // Set as current
        let mut current = self.current_identity.write().unwrap();
        *current = Some(identity.clone());
        
        Ok(identity)
    }
    
    /// Load an identity
    async fn load_identity(&self, id: &str) -> IdentityResult<Identity> {
        let identities = self.identities.read().unwrap();
        identities.get(id)
            .cloned()
            .ok_or(IdentityError::IdentityNotFound(id.to_string()))
    }
    
    /// Get all identities
    async fn get_all_identities(&self) -> IdentityResult<Vec<Identity>> {
        let identities = self.identities.read().unwrap();
        Ok(identities.values().cloned().collect())
    }
    
    /// Update an identity
    async fn update_identity(&self, identity: &Identity) -> IdentityResult<Identity> {
        let mut updated = identity.clone();
        updated.updated_at = timestamp_secs();
        
        let mut identities = self.identities.write().unwrap();
        
        if !identities.contains_key(&updated.id) {
            return Err(IdentityError::IdentityNotFound(updated.id.clone()));
        }
        
        identities.insert(updated.id.clone(), updated.clone());
        Ok(updated)
    }
    
    /// Delete an identity
    async fn delete_identity(&self, id: &str) -> IdentityResult<()> {
        let mut identities = self.identities.write().unwrap();
        
        if !identities.contains_key(id) {
            return Err(IdentityError::IdentityNotFound(id.to_string()));
        }
        
        identities.remove(id);
        Ok(())
    }
    
    /// Sign data
    async fn sign(&self, _data: &[u8]) -> IdentityResult<Vec<u8>> {
        // Mock implementation returns a simple signature
        Ok(vec![0, 1, 2, 3])
    }
    
    /// Verify a signature
    async fn verify(&self, _identity_id: &str, _data: &[u8], _signature: &[u8]) -> IdentityResult<bool> {
        // Use the always_verify flag to determine the result
        let always_verify = *self.always_verify.read().unwrap();
        Ok(always_verify)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_create_and_get_identity() {
        let provider = MockIdentityProvider::new();
        
        // Create a new identity
        let metadata = HashMap::new();
        let identity = provider.create_identity("Test User", metadata).await.unwrap();
        
        // Get the current identity
        let current = provider.get_identity().await.unwrap();
        
        // Verify it's the same
        assert_eq!(identity.id, current.id);
        assert_eq!(identity.name, current.name);
    }
    
    #[tokio::test]
    async fn test_sign_and_verify() {
        let provider = MockIdentityProvider::new();
        
        // Get the current identity
        let identity = provider.get_identity().await.unwrap();
        
        // Sign some data
        let data = b"test data";
        let signature = provider.sign(data).await.unwrap();
        
        // Verify the signature
        let verified = provider.verify(&identity.id, data, &signature).await.unwrap();
        assert!(verified);
    }
} 