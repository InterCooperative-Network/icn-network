//! Attestation module for identity verification
//!
//! This module provides functionality for creating and verifying attestations,
//! which are statements made by one identity about another.

use std::collections::HashMap;
use std::sync::Arc;
use std::fmt;
use async_trait::async_trait;
use thiserror::Error;
use serde::{Serialize, Deserialize};
use tokio::sync::RwLock;
use icn_core::{
    crypto::{NodeId, Signature, Hash, sha256},
    storage::{Storage, StorageResult, StorageError},
    utils::timestamp_secs,
};

use super::{Identity, IdentityProvider, IdentityError, IdentityResult};

/// Error types for attestation operations
#[derive(Error, Debug)]
pub enum AttestationError {
    /// Error with the identity system
    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),
    
    /// Error with storage
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Invalid attestation
    #[error("Invalid attestation: {0}")]
    InvalidAttestation(String),
    
    /// Attestation not found
    #[error("Attestation not found: {0}")]
    AttestationNotFound(String),
    
    /// Verification failed
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
}

/// Result type for attestation operations
pub type AttestationResult<T> = Result<T, AttestationError>;

/// Types of attestations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AttestationType {
    /// Attests that the subject is a member of a group
    Membership,
    /// Attests that the subject has a particular skill or ability
    Skill,
    /// Attests that the subject has contributed to a project
    Contribution,
    /// Attests that the subject is trusted by the issuer
    Trust,
    /// Attests that the subject is known to the issuer
    Identity,
    /// Attests that the subject controls a resource
    ResourceControl,
    /// A custom attestation type
    Custom(String),
}

impl ToString for AttestationType {
    fn to_string(&self) -> String {
        match self {
            Self::Membership => "membership".to_string(),
            Self::Skill => "skill".to_string(),
            Self::Contribution => "contribution".to_string(),
            Self::Trust => "trust".to_string(),
            Self::Identity => "identity".to_string(),
            Self::ResourceControl => "resource_control".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// An attestation made by one identity about another
#[derive(Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// Unique identifier for this attestation
    pub id: String,
    /// The identity making the attestation (issuer)
    pub issuer: NodeId,
    /// The identity being attested to (subject)
    pub subject: NodeId,
    /// The type of attestation
    pub attestation_type: AttestationType,
    /// The specific claim being made
    pub claim: String,
    /// Additional attributes for the attestation
    pub attributes: HashMap<String, String>,
    /// When the attestation was created
    pub created_at: u64,
    /// When the attestation expires (0 means never)
    pub expires_at: u64,
    /// The signature from the issuer
    pub signature: Signature,
}

impl Attestation {
    /// Create a new unsigned attestation
    pub fn new(
        issuer: NodeId,
        subject: NodeId,
        attestation_type: AttestationType,
        claim: String,
        attributes: HashMap<String, String>,
        expires_at: u64,
    ) -> Self {
        let created_at = timestamp_secs();
        let id = format!("att-{}-{}-{}", issuer, subject, created_at);
        
        Self {
            id,
            issuer,
            subject,
            attestation_type,
            claim,
            attributes,
            created_at,
            expires_at,
            signature: Signature(Vec::new()), // Placeholder, will be set when signed
        }
    }
    
    /// Get the bytes to sign for this attestation
    pub fn bytes_to_sign(&self) -> Vec<u8> {
        // Serialize the attestation data without the signature
        let serializable = AttestationData {
            id: self.id.clone(),
            issuer: self.issuer.clone(),
            subject: self.subject.clone(),
            attestation_type: self.attestation_type.clone(),
            claim: self.claim.clone(),
            attributes: self.attributes.clone(),
            created_at: self.created_at,
            expires_at: self.expires_at,
        };
        
        serde_json::to_vec(&serializable).unwrap_or_default()
    }
    
    /// Check if the attestation is valid and not expired
    pub fn is_valid(&self) -> bool {
        if self.expires_at > 0 {
            let now = timestamp_secs();
            if now > self.expires_at {
                return false;
            }
        }
        
        !self.signature.as_bytes().is_empty()
    }
}

impl fmt::Debug for Attestation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Attestation {{ id: {}, issuer: {}, subject: {}, type: {:?}, claim: {}, valid: {} }}",
            self.id, self.issuer, self.subject, self.attestation_type, self.claim, self.is_valid())
    }
}

/// Serializable attestation data for signing
#[derive(Serialize, Deserialize)]
struct AttestationData {
    /// Unique identifier for this attestation
    pub id: String,
    /// The identity making the attestation (issuer)
    pub issuer: NodeId,
    /// The identity being attested to (subject)
    pub subject: NodeId,
    /// The type of attestation
    pub attestation_type: AttestationType,
    /// The specific claim being made
    pub claim: String,
    /// Additional attributes for the attestation
    pub attributes: HashMap<String, String>,
    /// When the attestation was created
    pub created_at: u64,
    /// When the attestation expires (0 means never)
    pub expires_at: u64,
}

/// A trait for attestation verification
#[async_trait]
pub trait AttestationVerifier: Send + Sync {
    /// Verify an attestation's signature
    async fn verify_attestation(&self, attestation: &Attestation) -> AttestationResult<bool>;
    
    /// Verify that an attestation is valid and not expired
    async fn is_attestation_valid(&self, attestation: &Attestation) -> AttestationResult<bool>;
}

/// A manager for attestations
pub struct AttestationManager {
    /// The identity provider
    identity_provider: Arc<dyn IdentityProvider>,
    /// Storage for attestations
    storage: Arc<dyn Storage>,
    /// Cache of attestations (by ID)
    attestations_by_id: Arc<RwLock<HashMap<String, Attestation>>>,
    /// Cache of attestations by subject (subject ID -> vec of attestation IDs)
    attestations_by_subject: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Cache of attestations by issuer (issuer ID -> vec of attestation IDs)
    attestations_by_issuer: Arc<RwLock<HashMap<String, Vec<String>>>>,
}

impl AttestationManager {
    /// Create a new attestation manager
    pub async fn new(identity_provider: Arc<dyn IdentityProvider>, storage: Arc<dyn Storage>) -> Self {
        let manager = Self {
            identity_provider,
            storage,
            attestations_by_id: Arc::new(RwLock::new(HashMap::new())),
            attestations_by_subject: Arc::new(RwLock::new(HashMap::new())),
            attestations_by_issuer: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Load attestations from storage
        let _ = manager.load_attestations().await;
        
        manager
    }
    
    /// Load attestations from storage
    async fn load_attestations(&self) -> AttestationResult<()> {
        let dir = "attestations";
        
        // Check if the directory exists
        if let Ok(keys) = self.storage.list(dir).await {
            let mut attestations_by_id = self.attestations_by_id.write().await;
            let mut attestations_by_subject = self.attestations_by_subject.write().await;
            let mut attestations_by_issuer = self.attestations_by_issuer.write().await;
            
            for key in keys {
                if key.ends_with(".json") {
                    let attestation_result: StorageResult<Attestation> = self.storage.get_json(&key).await;
                    
                    if let Ok(attestation) = attestation_result {
                        // Index by ID
                        attestations_by_id.insert(attestation.id.clone(), attestation.clone());
                        
                        // Index by subject
                        let subject_id = attestation.subject.as_str().to_string();
                        attestations_by_subject
                            .entry(subject_id)
                            .or_insert_with(Vec::new)
                            .push(attestation.id.clone());
                        
                        // Index by issuer
                        let issuer_id = attestation.issuer.as_str().to_string();
                        attestations_by_issuer
                            .entry(issuer_id)
                            .or_insert_with(Vec::new)
                            .push(attestation.id.clone());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Save an attestation to storage
    async fn save_attestation(&self, attestation: &Attestation) -> AttestationResult<()> {
        let key = format!("attestations/{}.json", attestation.id);
        self.storage.put_json(&key, attestation).await?;
        Ok(())
    }
    
    /// Create and sign a new attestation
    pub async fn create_attestation(
        &self,
        subject: NodeId,
        attestation_type: AttestationType,
        claim: String,
        attributes: HashMap<String, String>,
        expires_in_seconds: u64,
    ) -> AttestationResult<Attestation> {
        // Get the current identity (issuer)
        let identity = self.identity_provider.get_identity().await?;
        
        // Calculate expiration time
        let expires_at = if expires_in_seconds > 0 {
            timestamp_secs() + expires_in_seconds
        } else {
            0 // Never expires
        };
        
        // Create unsigned attestation
        let mut attestation = Attestation::new(
            identity.id.clone(),
            subject,
            attestation_type,
            claim,
            attributes,
            expires_at,
        );
        
        // Sign the attestation
        let bytes_to_sign = attestation.bytes_to_sign();
        let signature = self.identity_provider.sign(&bytes_to_sign).await?;
        attestation.signature = signature;
        
        // Save to storage
        self.save_attestation(&attestation).await?;
        
        // Add to caches
        {
            let mut attestations_by_id = self.attestations_by_id.write().await;
            attestations_by_id.insert(attestation.id.clone(), attestation.clone());
        }
        
        {
            let mut attestations_by_subject = self.attestations_by_subject.write().await;
            attestations_by_subject
                .entry(attestation.subject.as_str().to_string())
                .or_insert_with(Vec::new)
                .push(attestation.id.clone());
        }
        
        {
            let mut attestations_by_issuer = self.attestations_by_issuer.write().await;
            attestations_by_issuer
                .entry(attestation.issuer.as_str().to_string())
                .or_insert_with(Vec::new)
                .push(attestation.id.clone());
        }
        
        Ok(attestation)
    }
    
    /// Get an attestation by ID
    pub async fn get_attestation(&self, id: &str) -> AttestationResult<Option<Attestation>> {
        // Check cache first
        {
            let attestations = self.attestations_by_id.read().await;
            if let Some(attestation) = attestations.get(id) {
                return Ok(Some(attestation.clone()));
            }
        }
        
        // Try to load from storage
        let key = format!("attestations/{}.json", id);
        match self.storage.get_json::<Attestation>(&key).await {
            Ok(attestation) => {
                // Add to cache
                {
                    let mut attestations = self.attestations_by_id.write().await;
                    attestations.insert(attestation.id.clone(), attestation.clone());
                }
                Ok(Some(attestation))
            },
            Err(_) => Ok(None),
        }
    }
    
    /// Get all attestations for a subject identity
    pub async fn get_attestations_for_subject(&self, subject_id: &NodeId) -> AttestationResult<Vec<Attestation>> {
        let mut attestations = Vec::new();
        let subject_key = subject_id.as_str().to_string();
        
        // Get attestation IDs from cache
        let ids = {
            let cache = self.attestations_by_subject.read().await;
            cache.get(&subject_key).cloned().unwrap_or_default()
        };
        
        // Load attestations by ID
        for id in ids {
            if let Some(attestation) = self.get_attestation(&id).await? {
                attestations.push(attestation);
            }
        }
        
        Ok(attestations)
    }
    
    /// Get all attestations issued by an identity
    pub async fn get_attestations_from_issuer(&self, issuer_id: &NodeId) -> AttestationResult<Vec<Attestation>> {
        let mut attestations = Vec::new();
        let issuer_key = issuer_id.as_str().to_string();
        
        // Get attestation IDs from cache
        let ids = {
            let cache = self.attestations_by_issuer.read().await;
            cache.get(&issuer_key).cloned().unwrap_or_default()
        };
        
        // Load attestations by ID
        for id in ids {
            if let Some(attestation) = self.get_attestation(&id).await? {
                attestations.push(attestation);
            }
        }
        
        Ok(attestations)
    }
    
    /// Delete an attestation
    pub async fn delete_attestation(&self, id: &str) -> AttestationResult<()> {
        // Get the attestation first to update caches
        let attestation = if let Some(att) = self.get_attestation(id).await? {
            att
        } else {
            return Err(AttestationError::AttestationNotFound(id.to_string()));
        };
        
        // Delete from storage
        let key = format!("attestations/{}.json", id);
        self.storage.delete(&key).await?;
        
        // Remove from caches
        {
            let mut attestations_by_id = self.attestations_by_id.write().await;
            attestations_by_id.remove(id);
        }
        
        {
            let mut attestations_by_subject = self.attestations_by_subject.write().await;
            let subject_key = attestation.subject.as_str().to_string();
            if let Some(ids) = attestations_by_subject.get_mut(&subject_key) {
                ids.retain(|a_id| a_id != id);
            }
        }
        
        {
            let mut attestations_by_issuer = self.attestations_by_issuer.write().await;
            let issuer_key = attestation.issuer.as_str().to_string();
            if let Some(ids) = attestations_by_issuer.get_mut(&issuer_key) {
                ids.retain(|a_id| a_id != id);
            }
        }
        
        Ok(())
    }
}

#[async_trait]
impl AttestationVerifier for AttestationManager {
    async fn verify_attestation(&self, attestation: &Attestation) -> AttestationResult<bool> {
        // Get the issuer's identity
        let issuer = self.identity_provider.get_identity_by_id(&attestation.issuer).await?
            .ok_or_else(|| AttestationError::VerificationFailed(
                format!("Issuer identity not found: {}", attestation.issuer)
            ))?;
        
        // Verify the signature
        let bytes_to_sign = attestation.bytes_to_sign();
        let result = self.identity_provider.verify(&attestation.issuer, &bytes_to_sign, &attestation.signature).await?;
        
        Ok(result)
    }
    
    async fn is_attestation_valid(&self, attestation: &Attestation) -> AttestationResult<bool> {
        // Check expiration
        if attestation.expires_at > 0 {
            let now = timestamp_secs();
            if now > attestation.expires_at {
                return Ok(false);
            }
        }
        
        // Verify signature
        self.verify_attestation(attestation).await
    }
} 