//! Storage implementation for identity data
//!
//! This module provides storage functionality for identity-related data.

use std::sync::Arc;
use async_trait::async_trait;
use icn_core::storage::{Storage, StorageError};
use crate::{Identity, IdentityResult, IdentityError};

/// Trait for identity storage operations
#[async_trait]
pub trait IdentityStorage: Send + Sync + 'static {
    /// Store an identity
    async fn store_identity(&self, identity: &Identity) -> IdentityResult<()>;
    
    /// Retrieve an identity by ID
    async fn get_identity(&self, id: &str) -> IdentityResult<Identity>;
    
    /// List all identities
    async fn list_identities(&self) -> IdentityResult<Vec<Identity>>;
    
    /// Delete an identity by ID
    async fn delete_identity(&self, id: &str) -> IdentityResult<()>;
}

/// Implementation of identity storage using the core storage layer
pub struct DefaultIdentityStorage {
    /// Underlying storage
    storage: Arc<dyn Storage>,
    /// Prefix for identity keys
    prefix: String,
}

impl DefaultIdentityStorage {
    /// Create a new identity storage instance
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            prefix: "identity:".to_string(),
        }
    }
    
    /// Get the storage key for an identity
    fn get_key(&self, id: &str) -> String {
        format!("{}{}", self.prefix, id)
    }
}

#[async_trait]
impl IdentityStorage for DefaultIdentityStorage {
    /// Store an identity
    async fn store_identity(&self, identity: &Identity) -> IdentityResult<()> {
        let key = self.get_key(&identity.id);
        
        // Serialize the identity to JSON
        let json = serde_json::to_string(identity)
            .map_err(|e| IdentityError::Other(format!("Serialization error: {}", e)))?;
        
        // Store in the underlying storage
        self.storage.put(&key, json.as_bytes()).await
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Retrieve an identity by ID
    async fn get_identity(&self, id: &str) -> IdentityResult<Identity> {
        let key = self.get_key(id);
        
        // Check if key exists
        let exists = self.storage.exists(&key).await
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        
        if !exists {
            return Err(IdentityError::IdentityNotFound(id.to_string()));
        }
        
        // Retrieve from storage
        let data = self.storage.get(&key).await
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        
        // Deserialize the identity
        let identity: Identity = serde_json::from_slice(&data)
            .map_err(|e| IdentityError::Other(format!("Deserialization error: {}", e)))?;
        
        Ok(identity)
    }
    
    /// List all identities
    async fn list_identities(&self) -> IdentityResult<Vec<Identity>> {
        // List all keys with our prefix
        let keys = self.storage.list(&self.prefix).await
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        
        let mut identities = Vec::new();
        
        // Retrieve each identity
        for key in keys {
            // Extract the ID from the key
            let id = key.strip_prefix(&self.prefix)
                .ok_or_else(|| IdentityError::Other(format!("Invalid key format: {}", key)))?;
            
            // Get the identity
            match self.get_identity(id).await {
                Ok(identity) => identities.push(identity),
                Err(IdentityError::IdentityNotFound(_)) => continue, // Skip not found
                Err(e) => return Err(e),
            }
        }
        
        Ok(identities)
    }
    
    /// Delete an identity by ID
    async fn delete_identity(&self, id: &str) -> IdentityResult<()> {
        let key = self.get_key(id);
        
        // Delete from storage
        self.storage.delete(&key).await
            .map_err(|e| IdentityError::StorageError(e.to_string()))?;
        
        Ok(())
    }
} 