use async_trait::async_trait;
use std::error::Error;
use serde::{Serialize, Deserialize};

/// Details of an identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityDetails {
    pub did: String,
    pub public_key: Vec<u8>,
    pub metadata: std::collections::HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Registration information for a new identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityRegistration {
    pub coop_id: String,
    pub node_id: String,
    pub public_key: Vec<u8>,
    pub metadata: std::collections::HashMap<String, String>,
}

/// Result type for identity operations
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Provider interface for identity-related operations
#[async_trait]
pub trait IdentityProvider: Send + Sync {
    /// Validate if the given DID represents a valid identity
    async fn validate_identity(&self, did: &str) -> Result<bool>;
    
    /// Retrieve detailed information about an identity
    async fn get_identity_details(&self, did: &str) -> Result<Option<IdentityDetails>>;
    
    /// Register a new identity in the system
    async fn register_identity(&self, details: IdentityRegistration) -> Result<String>;
    
    /// Verify if a signature was created by the identity
    async fn verify_signature(&self, did: &str, message: &[u8], signature: &[u8]) -> Result<bool>;
} 