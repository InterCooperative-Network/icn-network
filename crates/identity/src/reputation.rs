//! Reputation module for identity trust management
//!
//! This module provides functionality for managing identity reputation scores
//! and evidence in the InterCooperative Network.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
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
use super::attestation::{Attestation, AttestationVerifier, AttestationError};

/// Error types for reputation operations
#[derive(Error, Debug)]
pub enum ReputationError {
    /// Error with the identity system
    #[error("Identity error: {0}")]
    IdentityError(#[from] IdentityError),
    
    /// Error with storage
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Error with attestations
    #[error("Attestation error: {0}")]
    AttestationError(#[from] AttestationError),
    
    /// Invalid evidence
    #[error("Invalid evidence: {0}")]
    InvalidEvidence(String),
    
    /// Evidence not found
    #[error("Evidence not found: {0}")]
    EvidenceNotFound(String),
    
    /// Unauthorized action
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

/// Result type for reputation operations
pub type ReputationResult<T> = Result<T, ReputationError>;

/// Types of evidence that can affect reputation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EvidenceType {
    /// A successful transaction or interaction
    SuccessfulTransaction,
    /// A failed transaction or interaction
    FailedTransaction,
    /// Positive feedback from another identity
    PositiveFeedback,
    /// Negative feedback from another identity
    NegativeFeedback,
    /// Validation of some work or contribution
    Validation,
    /// Attestation from a trusted identity
    Attestation,
    /// Governance participation (voting, proposals)
    GovernanceParticipation,
    /// A custom evidence type
    Custom(String),
}

impl ToString for EvidenceType {
    fn to_string(&self) -> String {
        match self {
            Self::SuccessfulTransaction => "successful_transaction".to_string(),
            Self::FailedTransaction => "failed_transaction".to_string(),
            Self::PositiveFeedback => "positive_feedback".to_string(),
            Self::NegativeFeedback => "negative_feedback".to_string(),
            Self::Validation => "validation".to_string(),
            Self::Attestation => "attestation".to_string(),
            Self::GovernanceParticipation => "governance_participation".to_string(),
            Self::Custom(name) => name.clone(),
        }
    }
}

/// Evidence for reputation scoring
#[derive(Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// Unique identifier for this evidence
    pub id: String,
    /// The identity that submitted the evidence
    pub submitter: NodeId,
    /// The identity the evidence is about
    pub subject: NodeId,
    /// The type of evidence
    pub evidence_type: EvidenceType,
    /// A description of the evidence
    pub description: String,
    /// The weight of this evidence (-1.0 to 1.0)
    pub weight: f64,
    /// Associated data for the evidence
    pub data: HashMap<String, String>,
    /// When the evidence was created
    pub created_at: u64,
    /// References to related evidence
    pub references: Vec<String>,
    /// The signature from the submitter
    pub signature: Signature,
}

impl Evidence {
    /// Create a new unsigned evidence
    pub fn new(
        submitter: NodeId,
        subject: NodeId,
        evidence_type: EvidenceType,
        description: String,
        weight: f64,
        data: HashMap<String, String>,
        references: Vec<String>,
    ) -> Self {
        let created_at = timestamp_secs();
        let id = format!("evidence-{}-{}-{}", submitter, subject, created_at);
        
        Self {
            id,
            submitter,
            subject,
            evidence_type,
            description,
            weight: weight.max(-1.0).min(1.0), // Clamp to [-1.0, 1.0]
            data,
            created_at,
            references,
            signature: Signature(Vec::new()), // Placeholder, will be set when signed
        }
    }
    
    /// Get the bytes to sign for this evidence
    pub fn bytes_to_sign(&self) -> Vec<u8> {
        // Serialize the evidence data without the signature
        let serializable = EvidenceData {
            id: self.id.clone(),
            submitter: self.submitter.clone(),
            subject: self.subject.clone(),
            evidence_type: self.evidence_type.clone(),
            description: self.description.clone(),
            weight: self.weight,
            data: self.data.clone(),
            created_at: self.created_at,
            references: self.references.clone(),
        };
        
        serde_json::to_vec(&serializable).unwrap_or_default()
    }
}

impl std::fmt::Debug for Evidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Evidence {{ id: {}, submitter: {}, subject: {}, type: {:?}, weight: {} }}",
            self.id, self.submitter, self.subject, self.evidence_type, self.weight)
    }
}

/// Serializable evidence data for signing
#[derive(Serialize, Deserialize)]
struct EvidenceData {
    /// Unique identifier for this evidence
    pub id: String,
    /// The identity that submitted the evidence
    pub submitter: NodeId,
    /// The identity the evidence is about
    pub subject: NodeId,
    /// The type of evidence
    pub evidence_type: EvidenceType,
    /// A description of the evidence
    pub description: String,
    /// The weight of this evidence (-1.0 to 1.0)
    pub weight: f64,
    /// Associated data for the evidence
    pub data: HashMap<String, String>,
    /// When the evidence was created
    pub created_at: u64,
    /// References to related evidence
    pub references: Vec<String>,
}

/// A reputation score for an identity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReputationScore {
    /// The identity this score is for
    pub identity_id: NodeId,
    /// The overall score (0.0 to 1.0)
    pub score: f64,
    /// The number of positive evidence items
    pub positive_count: u32,
    /// The number of negative evidence items
    pub negative_count: u32,
    /// The total number of evidence items
    pub total_count: u32,
    /// Scores by category
    pub category_scores: HashMap<String, f64>,
    /// Last updated timestamp
    pub updated_at: u64,
}

impl ReputationScore {
    /// Create a new reputation score for an identity
    pub fn new(identity_id: NodeId) -> Self {
        Self {
            identity_id,
            score: 0.5, // Start at neutral score
            positive_count: 0,
            negative_count: 0,
            total_count: 0,
            category_scores: HashMap::new(),
            updated_at: timestamp_secs(),
        }
    }
    
    /// Apply evidence to this reputation score
    pub fn apply_evidence(&mut self, evidence: &Evidence) {
        // Simple reputation model:
        // - Each piece of evidence contributes to the overall score based on its weight
        // - Positive weights increase the score, negative weights decrease it
        // - The more evidence we have, the less impact each new piece has
        
        self.total_count += 1;
        
        if evidence.weight > 0.0 {
            self.positive_count += 1;
        } else if evidence.weight < 0.0 {
            self.negative_count += 1;
        }
        
        // Convert category to string for the category scores
        let category = evidence.evidence_type.to_string();
        
        // Update category score
        let category_score = self.category_scores.entry(category).or_insert(0.5);
        
        // Simple dampening function: the more evidence we have, the smaller the impact
        let dampening = 1.0 / (1.0 + (self.total_count as f64 * 0.1));
        let impact = evidence.weight * dampening;
        
        // Update category score (keeping it between 0 and 1)
        *category_score = (*category_score + impact).max(0.0).min(1.0);
        
        // Calculate overall score as average of category scores
        let sum: f64 = self.category_scores.values().sum();
        let count = self.category_scores.len().max(1) as f64;
        self.score = (sum / count).max(0.0).min(1.0);
        
        // Update timestamp
        self.updated_at = timestamp_secs();
    }
}

/// A trait for reputation
#[async_trait]
pub trait Reputation: Send + Sync {
    /// Get the reputation score for an identity
    async fn get_reputation(&self, identity_id: &NodeId) -> ReputationResult<ReputationScore>;
    
    /// Submit evidence about an identity
    async fn submit_evidence(&self, evidence: Evidence) -> ReputationResult<()>;
    
    /// Get evidence for an identity
    async fn get_evidence(&self, identity_id: &NodeId) -> ReputationResult<Vec<Evidence>>;
    
    /// Get a specific piece of evidence by ID
    async fn get_evidence_by_id(&self, evidence_id: &str) -> ReputationResult<Option<Evidence>>;
    
    /// Verify evidence signature
    async fn verify_evidence(&self, evidence: &Evidence) -> ReputationResult<bool>;
}

/// A manager for reputation evidence and scoring
pub struct ReputationManager {
    /// The identity provider
    identity_provider: Arc<dyn IdentityProvider>,
    /// Storage for reputation data
    storage: Arc<dyn Storage>,
    /// Attestation verifier for attestation-based evidence
    attestation_verifier: Option<Arc<dyn AttestationVerifier>>,
    /// Cache of reputation scores (by identity ID)
    reputation_scores: Arc<RwLock<HashMap<String, ReputationScore>>>,
    /// Cache of evidence (by ID)
    evidence_by_id: Arc<RwLock<HashMap<String, Evidence>>>,
    /// Cache of evidence by subject (subject ID -> HashSet of evidence IDs)
    evidence_by_subject: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Cache of evidence by submitter (submitter ID -> HashSet of evidence IDs)
    evidence_by_submitter: Arc<RwLock<HashMap<String, HashSet<String>>>>,
}

impl ReputationManager {
    /// Create a new reputation manager
    pub async fn new(
        identity_provider: Arc<dyn IdentityProvider>,
        storage: Arc<dyn Storage>,
        attestation_verifier: Option<Arc<dyn AttestationVerifier>>,
    ) -> Self {
        let manager = Self {
            identity_provider,
            storage,
            attestation_verifier,
            reputation_scores: Arc::new(RwLock::new(HashMap::new())),
            evidence_by_id: Arc::new(RwLock::new(HashMap::new())),
            evidence_by_subject: Arc::new(RwLock::new(HashMap::new())),
            evidence_by_submitter: Arc::new(RwLock::new(HashMap::new())),
        };
        
        // Load evidence from storage
        let _ = manager.load_evidence().await;
        
        // Calculate initial reputation scores
        let _ = manager.calculate_all_reputation_scores().await;
        
        manager
    }
    
    /// Load evidence from storage
    async fn load_evidence(&self) -> ReputationResult<()> {
        let dir = "evidence";
        
        // Check if the directory exists
        if let Ok(keys) = self.storage.list(dir).await {
            let mut evidence_by_id = self.evidence_by_id.write().await;
            let mut evidence_by_subject = self.evidence_by_subject.write().await;
            let mut evidence_by_submitter = self.evidence_by_submitter.write().await;
            
            for key in keys {
                if key.ends_with(".json") {
                    let evidence_result: StorageResult<Evidence> = self.storage.get_json(&key).await;
                    
                    if let Ok(evidence) = evidence_result {
                        // Index by ID
                        evidence_by_id.insert(evidence.id.clone(), evidence.clone());
                        
                        // Index by subject
                        let subject_id = evidence.subject.as_str().to_string();
                        evidence_by_subject
                            .entry(subject_id)
                            .or_insert_with(HashSet::new)
                            .insert(evidence.id.clone());
                        
                        // Index by submitter
                        let submitter_id = evidence.submitter.as_str().to_string();
                        evidence_by_submitter
                            .entry(submitter_id)
                            .or_insert_with(HashSet::new)
                            .insert(evidence.id.clone());
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Calculate reputation scores for all known identities
    async fn calculate_all_reputation_scores(&self) -> ReputationResult<()> {
        let subjects = {
            let evidence_by_subject = self.evidence_by_subject.read().await;
            evidence_by_subject.keys().cloned().collect::<Vec<_>>()
        };
        
        for subject_id in subjects {
            let _ = self.calculate_reputation_score(&subject_id).await;
        }
        
        Ok(())
    }
    
    /// Calculate the reputation score for a specific identity
    async fn calculate_reputation_score(&self, identity_id: &str) -> ReputationResult<ReputationScore> {
        // Get all evidence for this identity
        let evidence_ids = {
            let evidence_by_subject = self.evidence_by_subject.read().await;
            if let Some(ids) = evidence_by_subject.get(identity_id) {
                ids.clone()
            } else {
                HashSet::new()
            }
        };
        
        // Create a base reputation score
        let node_id = NodeId::from_string(identity_id);
        let mut score = ReputationScore::new(node_id);
        
        // Process each piece of evidence
        let evidence_by_id = self.evidence_by_id.read().await;
        for id in evidence_ids {
            if let Some(evidence) = evidence_by_id.get(&id) {
                score.apply_evidence(evidence);
            }
        }
        
        // Cache the score
        {
            let mut reputation_scores = self.reputation_scores.write().await;
            reputation_scores.insert(identity_id.to_string(), score.clone());
        }
        
        Ok(score)
    }
    
    /// Save a piece of evidence to storage
    async fn save_evidence(&self, evidence: &Evidence) -> ReputationResult<()> {
        let key = format!("evidence/{}.json", evidence.id);
        self.storage.put_json(&key, evidence).await?;
        Ok(())
    }
    
    /// Create and sign a new piece of evidence
    pub async fn create_evidence(
        &self,
        subject: NodeId,
        evidence_type: EvidenceType,
        description: String,
        weight: f64,
        data: HashMap<String, String>,
        references: Vec<String>,
    ) -> ReputationResult<Evidence> {
        // Get the current identity (submitter)
        let identity = self.identity_provider.get_identity().await?;
        
        // Create unsigned evidence
        let mut evidence = Evidence::new(
            identity.id.clone(),
            subject,
            evidence_type,
            description,
            weight,
            data,
            references,
        );
        
        // Sign the evidence
        let bytes_to_sign = evidence.bytes_to_sign();
        let signature = self.identity_provider.sign(&bytes_to_sign).await?;
        evidence.signature = signature;
        
        // Save to storage
        self.save_evidence(&evidence).await?;
        
        // Add to caches
        {
            let mut evidence_by_id = self.evidence_by_id.write().await;
            evidence_by_id.insert(evidence.id.clone(), evidence.clone());
        }
        
        {
            let mut evidence_by_subject = self.evidence_by_subject.write().await;
            evidence_by_subject
                .entry(evidence.subject.as_str().to_string())
                .or_insert_with(HashSet::new)
                .insert(evidence.id.clone());
        }
        
        {
            let mut evidence_by_submitter = self.evidence_by_submitter.write().await;
            evidence_by_submitter
                .entry(evidence.submitter.as_str().to_string())
                .or_insert_with(HashSet::new)
                .insert(evidence.id.clone());
        }
        
        // Update reputation score
        let subject_id = evidence.subject.as_str().to_string();
        let _ = self.calculate_reputation_score(&subject_id).await;
        
        Ok(evidence)
    }
    
    /// Create evidence based on an attestation
    pub async fn create_evidence_from_attestation(
        &self,
        attestation: &Attestation,
        weight: f64,
    ) -> ReputationResult<Evidence> {
        // Verify the attestation first
        if let Some(verifier) = &self.attestation_verifier {
            if !verifier.is_attestation_valid(attestation).await? {
                return Err(ReputationError::InvalidEvidence(
                    "Attestation is not valid".to_string()
                ));
            }
        }
        
        // Create evidence data from the attestation
        let mut data = HashMap::new();
        data.insert("attestation_id".to_string(), attestation.id.clone());
        data.insert("attestation_type".to_string(), attestation.attestation_type.to_string());
        data.insert("attestation_claim".to_string(), attestation.claim.clone());
        
        // Create the evidence
        self.create_evidence(
            attestation.subject.clone(),
            EvidenceType::Attestation,
            format!("Attestation: {}", attestation.claim),
            weight,
            data,
            Vec::new(),
        ).await
    }
}

#[async_trait]
impl Reputation for ReputationManager {
    async fn get_reputation(&self, identity_id: &NodeId) -> ReputationResult<ReputationScore> {
        let id_str = identity_id.as_str().to_string();
        
        // Check cache first
        {
            let scores = self.reputation_scores.read().await;
            if let Some(score) = scores.get(&id_str) {
                return Ok(score.clone());
            }
        }
        
        // Calculate reputation score
        self.calculate_reputation_score(&id_str).await
    }
    
    async fn submit_evidence(&self, evidence: Evidence) -> ReputationResult<()> {
        // Verify the evidence signature
        if !self.verify_evidence(&evidence).await? {
            return Err(ReputationError::InvalidEvidence(
                "Evidence signature verification failed".to_string()
            ));
        }
        
        // Save the evidence
        self.save_evidence(&evidence).await?;
        
        // Add to caches
        {
            let mut evidence_by_id = self.evidence_by_id.write().await;
            evidence_by_id.insert(evidence.id.clone(), evidence.clone());
        }
        
        {
            let mut evidence_by_subject = self.evidence_by_subject.write().await;
            evidence_by_subject
                .entry(evidence.subject.as_str().to_string())
                .or_insert_with(HashSet::new)
                .insert(evidence.id.clone());
        }
        
        {
            let mut evidence_by_submitter = self.evidence_by_submitter.write().await;
            evidence_by_submitter
                .entry(evidence.submitter.as_str().to_string())
                .or_insert_with(HashSet::new)
                .insert(evidence.id.clone());
        }
        
        // Update reputation score
        let subject_id = evidence.subject.as_str().to_string();
        let _ = self.calculate_reputation_score(&subject_id).await;
        
        Ok(())
    }
    
    async fn get_evidence(&self, identity_id: &NodeId) -> ReputationResult<Vec<Evidence>> {
        let mut evidence = Vec::new();
        let identity_key = identity_id.as_str().to_string();
        
        // Get evidence IDs from cache
        let ids = {
            let cache = self.evidence_by_subject.read().await;
            if let Some(ids) = cache.get(&identity_key) {
                ids.clone()
            } else {
                HashSet::new()
            }
        };
        
        // Load evidence by ID
        let evidence_by_id = self.evidence_by_id.read().await;
        for id in ids {
            if let Some(e) = evidence_by_id.get(&id) {
                evidence.push(e.clone());
            }
        }
        
        Ok(evidence)
    }
    
    async fn get_evidence_by_id(&self, evidence_id: &str) -> ReputationResult<Option<Evidence>> {
        // Check cache first
        {
            let evidence = self.evidence_by_id.read().await;
            if let Some(e) = evidence.get(evidence_id) {
                return Ok(Some(e.clone()));
            }
        }
        
        // Try to load from storage
        let key = format!("evidence/{}.json", evidence_id);
        match self.storage.get_json::<Evidence>(&key).await {
            Ok(evidence) => {
                // Add to cache
                {
                    let mut cache = self.evidence_by_id.write().await;
                    cache.insert(evidence.id.clone(), evidence.clone());
                }
                Ok(Some(evidence))
            },
            Err(_) => Ok(None),
        }
    }
    
    async fn verify_evidence(&self, evidence: &Evidence) -> ReputationResult<bool> {
        // Get the submitter's identity
        let submitter = self.identity_provider.get_identity_by_id(&evidence.submitter).await?
            .ok_or_else(|| ReputationError::InvalidEvidence(
                format!("Submitter identity not found: {}", evidence.submitter)
            ))?;
        
        // Verify the signature
        let bytes_to_sign = evidence.bytes_to_sign();
        let result = self.identity_provider.verify(&evidence.submitter, &bytes_to_sign, &evidence.signature).await?;
        
        Ok(result)
    }
} 