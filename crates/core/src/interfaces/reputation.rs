use async_trait::async_trait;
use std::error::Error;
use serde::{Serialize, Deserialize};

/// Trust score representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub score: f64,
    pub confidence: f64,
    pub last_updated: u64,
}

/// Evidence of an interaction that affects reputation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub source_did: String,
    pub target_did: String,
    pub attestation_type: AttestationType,
    pub timestamp: u64,
    pub metadata: std::collections::HashMap<String, String>,
    pub signature: Vec<u8>,
}

/// Types of attestations that can be made
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AttestationType {
    SuccessfulTransaction,
    FailedTransaction,
    DisputeResolution,
    GovernanceParticipation,
    ResourceProviding,
    IdentityVerification,
    Other(String),
}

/// Context for validation operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationContext {
    pub action_type: String,
    pub resource_requirements: Option<std::collections::HashMap<String, f64>>,
    pub minimum_trust: Option<f64>,
}

/// Response from a validation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResponse {
    pub is_valid: bool,
    pub trust_score: Option<TrustScore>,
    pub reason: Option<String>,
}

/// Result type for reputation operations
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Provider interface for reputation-related operations
#[async_trait]
pub trait ReputationProvider: Send + Sync {
    /// Get the trust score for an entity
    async fn get_trust_score(&self, did: &str) -> Result<Option<TrustScore>>;
    
    /// Record an interaction that affects reputation
    async fn record_interaction(&self, evidence: Evidence) -> Result<()>;
    
    /// Validate if an entity meets trust requirements for a specific context
    async fn validate_entity(&self, did: &str, context: ValidationContext) -> Result<ValidationResponse>;
    
    /// Get reputation metrics for a specific time period
    async fn get_reputation_metrics(&self, start_time: u64, end_time: u64) -> Result<std::collections::HashMap<String, f64>>;
} 