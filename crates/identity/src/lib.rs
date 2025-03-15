//! Identity management for ICN
//!
//! This crate provides identity management capabilities for the InterCooperative
//! Network, including digital identity, reputation, and attestations.

use std::fmt;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use icn_core::{
    crypto::{IdentityKeyPair, NodeId, Signature, Hash, CryptoResult, CryptoError},
    storage::{Storage, StorageResult, StorageError, FileStorage},
};

/// Error types for identity operations
#[derive(Error, Debug)]
pub enum IdentityError {
    /// Error related to cryptography
    #[error("Crypto error: {0}")]
    CryptoError(#[from] CryptoError),
    
    /// Error related to storage
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Error related to verification
    #[error("Verification error: {0}")]
    VerificationError(String),
    
    /// Identity not found
    #[error("Identity not found: {0}")]
    IdentityNotFound(String),
    
    /// No current identity
    #[error("No current identity")]
    NoIdentity,
    
    /// Invalid identity data
    #[error("Invalid identity data: {0}")]
    InvalidIdentityData(String),
    
    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
}

/// Result type for identity operations
pub type IdentityResult<T> = Result<T, IdentityError>;

/// An identity in the network
#[derive(Clone, Serialize, Deserialize)]
pub struct Identity {
    /// Unique identifier for this identity
    pub id: String,
    
    /// Human-readable name
    pub name: String,
    
    /// Public key for this identity
    pub public_key: Vec<u8>,
    
    /// Additional metadata for this identity
    pub metadata: HashMap<String, String>,
    
    /// When the identity was created
    pub created_at: u64,
    
    /// When the identity was last updated
    pub updated_at: u64,
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Identity")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// The identity provider interface
#[async_trait]
pub trait IdentityProvider: Send + Sync + 'static {
    /// Get the current identity
    async fn get_identity(&self) -> IdentityResult<Identity>;
    
    /// Create a new identity
    async fn create_identity(&self, name: &str, metadata: HashMap<String, String>) -> IdentityResult<Identity>;
    
    /// Load an existing identity
    async fn load_identity(&self, id: &str) -> IdentityResult<Identity>;
    
    /// Get all identities
    async fn get_all_identities(&self) -> IdentityResult<Vec<Identity>>;
    
    /// Update an identity's metadata
    async fn update_identity(&self, identity: &Identity) -> IdentityResult<Identity>;
    
    /// Delete an identity
    async fn delete_identity(&self, id: &str) -> IdentityResult<()>;
    
    /// Sign data with the current identity
    async fn sign(&self, data: &[u8]) -> IdentityResult<Signature>;
    
    /// Verify a signature against an identity
    async fn verify(&self, identity_id: &str, data: &[u8], signature: &[u8]) -> IdentityResult<bool>;
}

/// Storage for identities
pub mod storage;

/// The main identity manager
pub struct IdentityManager {
    /// Storage for identities
    storage: Arc<dyn Storage>,
    /// Current identity key pair
    key_pair: Arc<RwLock<Option<IdentityKeyPair>>>,
    /// Current identity
    current_identity: Arc<RwLock<Option<Identity>>>,
}

impl IdentityManager {
    /// Create a new identity manager
    pub async fn new(
        storage: Arc<dyn Storage>,
        key_path: Option<PathBuf>,
    ) -> IdentityResult<Self> {
        let key_pair = if let Some(path) = key_path {
            // Load existing key pair
            match IdentityKeyPair::load_from_file(&path).await {
                Ok(kp) => Some(kp),
                Err(_) => {
                    // Generate new key pair and save it
                    let kp = IdentityKeyPair::generate()?;
                    kp.save_to_file(&path).await?;
                    Some(kp)
                }
            }
        } else {
            // Generate a temporary key pair
            Some(IdentityKeyPair::generate()?)
        };
        
        let manager = Self {
            storage,
            key_pair: Arc::new(RwLock::new(key_pair)),
            current_identity: Arc::new(RwLock::new(None)),
        };
        
        // Try to load the identity from storage
        if let Some(kp) = &*manager.key_pair.read().await {
            let id = kp.public_key_hash().to_string();
            let storage_key = format!("identity:{}", id);
            
            match manager.storage.get_json::<Identity>(&storage_key).await {
                Ok(identity) => {
                    let mut current = manager.current_identity.write().await;
                    *current = Some(identity);
                },
                Err(_) => {
                    // No identity found, but that's ok
                }
            }
        }
        
        Ok(manager)
    }
}

#[async_trait]
impl IdentityProvider for IdentityManager {
    async fn get_identity(&self) -> IdentityResult<Identity> {
        let current = self.current_identity.read().await;
        
        if let Some(identity) = current.clone() {
            Ok(identity)
        } else {
            Err(IdentityError::NoIdentity)
        }
    }
    
    async fn create_identity(&self, name: &str, metadata: HashMap<String, String>) -> IdentityResult<Identity> {
        // Ensure we have a key pair
        let key_pair = {
            let key_pair = self.key_pair.read().await;
            key_pair.clone().ok_or(IdentityError::CryptoError(CryptoError::KeyError("No key pair available".to_string())))?
        };
        
        // Create the identity
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let id = key_pair.public_key_hash().to_string();
        
        let identity = Identity {
            id: id.clone(),
            name: name.to_string(),
            public_key: key_pair.public_key().to_vec(),
            metadata,
            created_at: now,
            updated_at: now,
        };
        
        // Store the identity
        let storage_key = format!("identity:{}", id);
        self.storage.put_json(&storage_key, &identity).await?;
        
        // Set as current identity
        let mut current = self.current_identity.write().await;
        *current = Some(identity.clone());
        
        Ok(identity)
    }
    
    async fn load_identity(&self, id: &str) -> IdentityResult<Identity> {
        let storage_key = format!("identity:{}", id);
        match self.storage.get_json::<Identity>(&storage_key).await {
            Ok(identity) => Ok(identity),
            Err(_) => Err(IdentityError::IdentityNotFound(id.to_string())),
        }
    }
    
    async fn get_all_identities(&self) -> IdentityResult<Vec<Identity>> {
        let keys = self.storage.list("identity:").await?;
        let mut identities = Vec::new();
        
        for key in keys {
            match self.storage.get_json::<Identity>(&key).await {
                Ok(identity) => identities.push(identity),
                Err(_) => continue,
            }
        }
        
        Ok(identities)
    }
    
    async fn update_identity(&self, identity: &Identity) -> IdentityResult<Identity> {
        // Check if the identity exists
        let storage_key = format!("identity:{}", identity.id);
        let _ = self.storage.get_json::<Identity>(&storage_key).await
            .map_err(|_| IdentityError::IdentityNotFound(identity.id.clone()))?;
        
        // Create updated identity
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut updated = identity.clone();
        updated.updated_at = now;
        
        // Store the updated identity
        self.storage.put_json(&storage_key, &updated).await?;
        
        // Update current identity if needed
        let current = self.current_identity.read().await;
        if let Some(current_identity) = &*current {
            if current_identity.id == identity.id {
                let mut current = self.current_identity.write().await;
                *current = Some(updated.clone());
            }
        }
        
        Ok(updated)
    }
    
    async fn delete_identity(&self, id: &str) -> IdentityResult<()> {
        let storage_key = format!("identity:{}", id);
        
        // Check if the identity exists
        let _ = self.storage.get_json::<Identity>(&storage_key).await
            .map_err(|_| IdentityError::IdentityNotFound(id.to_string()))?;
        
        // Delete the identity
        self.storage.delete(&storage_key).await?;
        
        // Update current identity if needed
        let current = self.current_identity.read().await;
        if let Some(current_identity) = &*current {
            if current_identity.id == id {
                let mut current = self.current_identity.write().await;
                *current = None;
            }
        }
        
        Ok(())
    }
    
    async fn sign(&self, data: &[u8]) -> IdentityResult<Signature> {
        let key_pair = self.key_pair.read().await;
        let key_pair = key_pair.as_ref()
            .ok_or(IdentityError::CryptoError(CryptoError::KeyError("No key pair available".to_string())))?;
        
        let signature = key_pair.sign(data)?;
        Ok(signature)
    }
    
    async fn verify(&self, identity_id: &str, data: &[u8], signature: &[u8]) -> IdentityResult<bool> {
        // Load the identity
        let identity = self.load_identity(identity_id).await?;
        
        // Verify the signature
        let signature = Signature::from_bytes(signature)
            .map_err(|e| IdentityError::CryptoError(e))?;
        
        signature.verify(&identity.public_key, data)
            .map_err(|e| IdentityError::CryptoError(e))
            .or_else(|_| Ok(false))
    }
}

pub mod reputation;
pub mod attestation;

// Mock implementation for testing
#[cfg(any(test, feature = "testing"))]
pub mod mock;
#[cfg(any(test, feature = "testing"))]
pub use mock::MockIdentityProvider;

// Re-exports
pub use reputation::{Reputation, ReputationScore, ReputationManager};
pub use attestation::{Attestation, AttestationManager, AttestationVerifier};

// ICN Identity crate

//! Identity system for ICN, including DIDs and credentials.

/// Identity types and utilities
pub mod identity {
    /// A simple identity struct
    #[derive(Debug, Clone)]
    pub struct Identity {
        /// The identifier for this identity
        pub id: String,
    }

    impl Identity {
        /// Create a new identity
        pub fn new(id: &str) -> Self {
            Self {
                id: id.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_identity_creation() {
        let identity = identity::Identity::new("test-id");
        assert_eq!(identity.id, "test-id");
    }
} 