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
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

/// Result type for identity operations
pub type IdentityResult<T> = Result<T, IdentityError>;

/// A trait for identity providers
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Get the current identity
    async fn get_identity(&self) -> IdentityResult<Identity>;
    
    /// Get a list of known identities
    async fn get_known_identities(&self) -> IdentityResult<Vec<Identity>>;
    
    /// Get an identity by ID
    async fn get_identity_by_id(&self, id: &NodeId) -> IdentityResult<Option<Identity>>;
    
    /// Add a new known identity
    async fn add_identity(&self, identity: Identity) -> IdentityResult<()>;
    
    /// Create a signature for data
    async fn sign(&self, data: &[u8]) -> IdentityResult<Signature>;
    
    /// Verify a signature against an identity
    async fn verify(&self, identity_id: &NodeId, data: &[u8], signature: &Signature) -> IdentityResult<bool>;
}

/// An identity in the ICN network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// The node ID
    pub id: NodeId,
    /// The public key
    pub public_key: Vec<u8>,
    /// The name or alias
    pub name: String,
    /// The creation timestamp
    pub created_at: u64,
    /// Last updated timestamp
    pub updated_at: u64,
    /// Metadata associated with this identity
    pub metadata: HashMap<String, String>,
    /// Current reputation score (optional)
    pub reputation_score: Option<f64>,
}

/// A local identity provider that manages the node's identity
pub struct LocalIdentityProvider {
    /// The key pair for the local identity
    key_pair: Arc<RwLock<IdentityKeyPair>>,
    /// Storage for identity information
    storage: Arc<dyn Storage>,
    /// Known identities cache
    known_identities: Arc<RwLock<HashMap<String, Identity>>>,
}

impl LocalIdentityProvider {
    /// Create a new local identity provider
    pub async fn new(key_pair: IdentityKeyPair, storage: Arc<dyn Storage>) -> Self {
        let provider = Self {
            key_pair: Arc::new(RwLock::new(key_pair)),
            storage,
            known_identities: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Load known identities
        let _ = provider.load_known_identities().await;
        
        provider
    }
    
    /// Load known identities from storage
    async fn load_known_identities(&self) -> IdentityResult<()> {
        let ids_dir = "identities";
        
        // Check if the directory exists
        if let Ok(keys) = self.storage.list(ids_dir).await {
            let mut known_identities = self.known_identities.write().await;
            
            for key in keys {
                if key.ends_with(".json") {
                    let identity_result: StorageResult<Identity> = self.storage.get_json(&key).await;
                    
                    if let Ok(identity) = identity_result {
                        known_identities.insert(identity.id.as_str().to_string(), identity);
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Save a known identity to storage
    async fn save_identity(&self, identity: &Identity) -> IdentityResult<()> {
        let key = format!("identities/{}.json", identity.id.as_str());
        self.storage.put_json(&key, identity).await?;
        Ok(())
    }
    
    /// Create a local identity provider from a key file
    pub async fn from_key_file<P: AsRef<Path>>(key_path: P, storage_path: P) -> IdentityResult<Self> {
        let key_pair = IdentityKeyPair::load_from_file(key_path).await
            .map_err(|e| IdentityError::CryptoError(e))?;
        
        let storage = FileStorage::new(storage_path.as_ref()).await
            .map_err(|e| IdentityError::StorageError(e))?;
        
        Ok(Self::new(key_pair, Arc::new(storage)).await)
    }
    
    /// Create a new local identity
    pub async fn create_new<P: AsRef<Path>>(name: &str, key_path: P, storage_path: P) -> IdentityResult<Self> {
        let key_pair = IdentityKeyPair::generate()
            .map_err(|e| IdentityError::CryptoError(e))?;
        
        // Save the key pair to file
        key_pair.save_to_file(&key_path).await
            .map_err(|e| IdentityError::CryptoError(e))?;
        
        let storage = FileStorage::new(storage_path.as_ref()).await
            .map_err(|e| IdentityError::StorageError(e))?;
        
        let provider = Self::new(key_pair, Arc::new(storage)).await;
        
        // Create identity record
        let key_pair = provider.key_pair.read().await;
        let id = key_pair.node_id().clone();
        let timestamp = icn_core::utils::timestamp_secs();
        
        let identity = Identity {
            id: id.clone(),
            public_key: key_pair.public_key_bytes().to_vec(),
            name: name.to_string(),
            created_at: timestamp,
            updated_at: timestamp,
            metadata: HashMap::new(),
            reputation_score: Some(0.0),
        };
        
        // Save the identity
        provider.save_identity(&identity).await?;
        
        // Add to known identities
        {
            let mut known = provider.known_identities.write().await;
            known.insert(id.as_str().to_string(), identity);
        }
        
        Ok(provider)
    }
}

#[async_trait]
impl IdentityProvider for LocalIdentityProvider {
    async fn get_identity(&self) -> IdentityResult<Identity> {
        let key_pair = self.key_pair.read().await;
        let id = key_pair.node_id();
        
        let known = self.known_identities.read().await;
        if let Some(identity) = known.get(id.as_str()) {
            return Ok(identity.clone());
        }
        
        // Identity not in cache, try to load from storage
        let key = format!("identities/{}.json", id.as_str());
        match self.storage.get_json::<Identity>(&key).await {
            Ok(identity) => Ok(identity),
            Err(_) => {
                // Create a default identity if it doesn't exist
                let timestamp = icn_core::utils::timestamp_secs();
                let identity = Identity {
                    id: id.clone(),
                    public_key: key_pair.public_key_bytes().to_vec(),
                    name: format!("Node {}", id),
                    created_at: timestamp,
                    updated_at: timestamp,
                    metadata: HashMap::new(),
                    reputation_score: Some(0.0),
                };
                
                // Save the identity
                self.save_identity(&identity).await?;
                
                // Add to known identities
                {
                    let mut known = self.known_identities.write().await;
                    known.insert(id.as_str().to_string(), identity.clone());
                }
                
                Ok(identity)
            }
        }
    }
    
    async fn get_known_identities(&self) -> IdentityResult<Vec<Identity>> {
        let known = self.known_identities.read().await;
        Ok(known.values().cloned().collect())
    }
    
    async fn get_identity_by_id(&self, id: &NodeId) -> IdentityResult<Option<Identity>> {
        let known = self.known_identities.read().await;
        if let Some(identity) = known.get(id.as_str()) {
            return Ok(Some(identity.clone()));
        }
        
        // Try to load from storage
        let key = format!("identities/{}.json", id.as_str());
        match self.storage.get_json::<Identity>(&key).await {
            Ok(identity) => {
                // Add to cache
                {
                    let mut known = self.known_identities.write().await;
                    known.insert(id.as_str().to_string(), identity.clone());
                }
                Ok(Some(identity))
            },
            Err(_) => Ok(None),
        }
    }
    
    async fn add_identity(&self, identity: Identity) -> IdentityResult<()> {
        // Validate the identity (check that the ID matches the public key)
        let expected_id = NodeId::from_public_key(&identity.public_key);
        if identity.id != expected_id {
            return Err(IdentityError::VerificationError(
                format!("Identity ID does not match public key: {} != {}", identity.id, expected_id)
            ));
        }
        
        // Save to storage
        self.save_identity(&identity).await?;
        
        // Add to cache
        {
            let mut known = self.known_identities.write().await;
            known.insert(identity.id.as_str().to_string(), identity);
        }
        
        Ok(())
    }
    
    async fn sign(&self, data: &[u8]) -> IdentityResult<Signature> {
        let key_pair = self.key_pair.read().await;
        Ok(key_pair.sign(data))
    }
    
    async fn verify(&self, identity_id: &NodeId, data: &[u8], signature: &Signature) -> IdentityResult<bool> {
        // Get the identity
        let identity = self.get_identity_by_id(identity_id).await?
            .ok_or_else(|| IdentityError::IdentityNotFound(identity_id.to_string()))?;
        
        // Verify the signature
        icn_core::crypto::verify_signature(&identity.public_key, data, signature)
            .map(|_| true)
            .or_else(|_| Ok(false))
    }
}

pub mod reputation;
pub mod attestation;

// Re-exports
pub use reputation::{Reputation, ReputationScore, ReputationManager};
pub use attestation::{Attestation, AttestationManager, AttestationVerifier}; 