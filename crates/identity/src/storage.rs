//! Storage implementation for identity data
//!
//! This module provides storage functionality for identity-related data.

use std::sync::Arc;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use icn_core::storage::{Storage, StorageResult, StorageError};
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
    async fn store_identity(&self, identity: &Identity) -> IdentityResult<()> {
        let key = self.get_key(&identity.id);
        let data = serde_json::to_vec(identity)
            .map_err(|e| IdentityError::InvalidIdentityData(format!("Serialization error: {}", e)))?;
        
        self.storage.put(&key, &data).await
            .map_err(|e| IdentityError::StorageError(e))?;
        
        Ok(())
    }
    
    async fn get_identity(&self, id: &str) -> IdentityResult<Identity> {
        let key = self.get_key(id);
        let data = self.storage.get(&key).await
            .map_err(|e| match e {
                StorageError::KeyNotFound(_) => IdentityError::IdentityNotFound(id.to_string()),
                _ => IdentityError::StorageError(e),
            })?;
        
        let identity = serde_json::from_slice(&data)
            .map_err(|e| IdentityError::InvalidIdentityData(format!("Deserialization error: {}", e)))?;
        
        Ok(identity)
    }
    
    async fn list_identities(&self) -> IdentityResult<Vec<Identity>> {
        let keys = self.storage.list(&self.prefix).await
            .map_err(|e| IdentityError::StorageError(e))?;
        
        let mut identities = Vec::new();
        for key in keys {
            if let Ok(data) = self.storage.get(&key).await {
                if let Ok(identity) = serde_json::from_slice::<Identity>(&data) {
                    identities.push(identity);
                }
            }
        }
        
        Ok(identities)
    }
    
    async fn delete_identity(&self, id: &str) -> IdentityResult<()> {
        let key = self.get_key(id);
        self.storage.delete(&key).await
            .map_err(|e| IdentityError::StorageError(e))?;
        
        Ok(())
    }
} 