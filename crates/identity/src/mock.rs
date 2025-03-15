//! Mock identity provider for testing
//!
//! This module provides a mock implementation of the IdentityProvider
//! trait for use in tests.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde::{Serialize, Deserialize};

use icn_core::{
    crypto::{NodeId, KeyPair, Signature, Hash},
    utils::timestamp_secs,
};

use crate::{
    Identity, IdentityProvider, IdentityResult, IdentityError,
    storage::IdentityStorage,
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
        let mut identities = self.identities.write().unwrap();
        
        // Check if identity exists
        if !identities.contains_key(&identity.id) {
            return Err(IdentityError::IdentityNotFound(identity.id.clone()));
        }
        
        // Update the identity
        let mut updated = identity.clone();
        updated.updated_at = timestamp_secs();
        identities.insert(identity.id.clone(), updated.clone());
        
        // Update current if needed
        let current = self.current_identity.read().unwrap();
        if let Some(current_id) = current.as_ref() {
            if current_id.id == identity.id {
                let mut current = self.current_identity.write().unwrap();
                *current = Some(updated.clone());
            }
        }
        
        Ok(updated)
    }
    
    /// Delete an identity
    async fn delete_identity(&self, id: &str) -> IdentityResult<()> {
        let mut identities = self.identities.write().unwrap();
        
        // Check if identity exists
        if !identities.contains_key(id) {
            return Err(IdentityError::IdentityNotFound(id.to_string()));
        }
        
        // Remove the identity
        identities.remove(id);
        
        // Update current if needed
        let current = self.current_identity.read().unwrap();
        if let Some(current_id) = current.as_ref() {
            if current_id.id == id {
                let mut current = self.current_identity.write().unwrap();
                *current = None;
            }
        }
        
        Ok(())
    }
    
    /// Sign data with the current identity
    async fn sign(&self, data: &[u8]) -> IdentityResult<Signature> {
        // Mock signature just returns the first few bytes of the data
        let mut signature = Vec::new();
        signature.extend_from_slice(if data.len() > 8 { &data[0..8] } else { data });
        Ok(Signature(signature))
    }
    
    /// Verify a signature
    async fn verify(&self, identity_id: &str, data: &[u8], signature: &[u8]) -> IdentityResult<bool> {
        // Check if we're configured to always verify
        let always_verify = self.always_verify.read().unwrap();
        if *always_verify {
            return Ok(true);
        }
        
        // In a real implementation, we would:
        // 1. Get the identity's public key
        // 2. Use it to verify the signature against the data
        
        // For this mock, we'll just check if the signature starts with the data's first few bytes
        if data.len() > 0 && signature.len() >= 8 {
            let prefix_len = std::cmp::min(8, data.len());
            Ok(&data[0..prefix_len] == &signature[0..prefix_len])
        } else {
            Ok(false)
        }
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
        let verified = provider.verify(&identity.id, data, &signature.0).await.unwrap();
        assert!(verified);
    }
} 