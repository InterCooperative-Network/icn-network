use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use icn_core::storage::Storage;
use icn_core::identity::{Identity, DidDocument};
use icn_core::crypto::CryptoUtils;
use crate::NetworkError;

/// Errors that can occur in reputation operations
#[derive(Debug, Error)]
pub enum ReputationError {
    #[error("Invalid attestation: {0}")]
    InvalidAttestation(String),
    
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    
    #[error("Attestation not found: {0}")]
    AttestationNotFound(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Invalid score: {0}")]
    InvalidScore(String),
    
    #[error("Sybil attack detected: {0}")]
    SybilDetected(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for reputation operations
pub type ReputationResult<T> = Result<T, ReputationError>;

/// Type conversion from ReputationError to NetworkError
impl From<ReputationError> for NetworkError {
    fn from(err: ReputationError) -> Self {
        NetworkError::ReputationError(err.to_string())
    }
}

/// Attestation types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AttestationType {
    /// General cooperative identity verification
    CooperativeVerification,
    /// Member attestation by a cooperative
    MemberVerification,
    /// Transaction capability/trust
    TransactionTrust,
    /// Governance participation quality
    GovernanceQuality,
    /// Resource sharing reliability
    ResourceReliability,
    /// General trust attestation
    GeneralTrust,
}

/// Evidence for attestations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: String,
    pub evidence_id: String,
    pub description: String,
    pub timestamp: u64,
    pub data: Option<serde_json::Value>,
}

/// Multi-party signature for attestations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPartySignature {
    pub signer_did: String,
    pub signature: Vec<u8>,
    pub timestamp: u64,
    pub is_revoked: bool,
}

/// Core attestation structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    pub id: String,
    pub issuer_did: String,
    pub subject_did: String,
    pub attestation_type: AttestationType,
    pub score: f64, // Value between 0.0 and 1.0
    pub context: Vec<String>,
    pub claims: serde_json::Value,
    pub evidence: Vec<Evidence>,
    pub signatures: Vec<MultiPartySignature>,
    pub quorum_threshold: u32, // Minimum number of signatures required
    pub created_at: u64,
    pub expires_at: Option<u64>,
    pub is_revoked: bool,
}

/// Trust score with detailed components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub overall_score: f64, // Value between 0.0 and 1.0
    pub components: HashMap<String, f64>,
    pub attestation_count: usize,
    pub calculation_time: u64,
    pub confidence: f64, // How confident we are in this score
}

/// The main attestation manager
pub struct AttestationManager {
    identity: Arc<Identity>,
    storage: Arc<dyn Storage>,
    crypto: Arc<CryptoUtils>,
}

impl AttestationManager {
    pub fn new(
        identity: Arc<Identity>,
        storage: Arc<dyn Storage>,
        crypto: Arc<CryptoUtils>,
    ) -> Self {
        AttestationManager {
            identity,
            storage,
            crypto,
        }
    }

    /// Create a new attestation with an optional quorum requirement
    pub fn create_attestation(
        &self,
        subject_did: &str,
        attestation_type: AttestationType,
        score: f64,
        claims: serde_json::Value,
        evidence: Vec<Evidence>,
        quorum_threshold: u32,
        expiration_days: Option<u64>,
    ) -> ReputationResult<Attestation> {
        // Validate score range
        if score < 0.0 || score > 1.0 {
            return Err(ReputationError::InvalidScore(
                "Score must be between 0.0 and 1.0".to_string()),
            );
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ReputationError::Other(e.to_string()))?
            .as_secs();

        let issuer_did = self.identity.did.clone();
        
        // Generate a unique ID for the attestation
        let id = format!("att:{}:{}:{}", issuer_did, subject_did, now);
        
        // Calculate expiration
        let expires_at = expiration_days.map(|days| now + (days * 24 * 60 * 60));
        
        // Create signature data
        let signature_data = format!(
            "{}:{}:{}:{}", 
            id, 
            subject_did,
            now,
            score
        );
        
        // Create signature using the identity's private key
        let signature = self.identity.sign(signature_data.as_bytes())
            .map_err(|e| ReputationError::VerificationFailed(e.to_string()))?;
        
        let initial_signature = MultiPartySignature {
            signer_did: issuer_did.clone(),
            signature: signature.to_bytes().to_vec(),
            timestamp: now,
            is_revoked: false,
        };
        
        let attestation = Attestation {
            id,
            issuer_did,
            subject_did: subject_did.to_string(),
            attestation_type,
            score,
            context: vec!["https://schema.icn.coop/attestation/v1".to_string()],
            claims,
            evidence,
            signatures: vec![initial_signature],
            quorum_threshold,
            created_at: now,
            expires_at,
            is_revoked: false,
        };
        
        // Store the attestation
        tracing::info!("Storing attestation at: attestations/{}", attestation.id);
        self.storage.put_json(&format!("attestations/{}", attestation.id), &attestation)
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
        
        Ok(attestation)
    }
    
    /// Add a signature to an attestation (for multi-party attestations)
    pub fn sign_attestation(
        &self,
        attestation_id: &str,
        signer_did: &str,
        signature: Vec<u8>,
    ) -> ReputationResult<Attestation> {
        // Load the attestation
        let mut attestation: Attestation = self.storage.get_json(&format!("attestations/{}", attestation_id))
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
        
        // Verify that the attestation is not expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ReputationError::Other(e.to_string()))?
            .as_secs();
            
        if let Some(expires_at) = attestation.expires_at {
            if now > expires_at {
                return Err(ReputationError::InvalidAttestation(
                    "Attestation has expired".to_string()),
                );
            }
        }
        
        // Verify that the attestation is not revoked
        if attestation.is_revoked {
            return Err(ReputationError::InvalidAttestation(
                "Attestation has been revoked".to_string()),
            );
        }
        
        // Add the new signature
        let new_signature = MultiPartySignature {
            signer_did: signer_did.to_string(),
            signature,
            timestamp: now,
            is_revoked: false,
        };
        
        attestation.signatures.push(new_signature);
        
        // Update the attestation in storage
        self.storage.put_json(&format!("attestations/{}", attestation.id), &attestation)
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
        
        Ok(attestation)
    }
    
    /// Check if an attestation has reached its quorum threshold
    pub fn has_reached_quorum(&self, attestation: &Attestation) -> bool {
        let valid_signatures = attestation.signatures.iter()
            .filter(|sig| !sig.is_revoked)
            .count() as u32;
            
        valid_signatures >= attestation.quorum_threshold
    }
    
    /// Get all attestations for a subject
    pub fn get_attestations_for_subject(&self, subject_did: &str) -> ReputationResult<Vec<Attestation>> {
        let prefix = format!("attestations/att:*:{}:*", subject_did);
        let keys = self.storage.list(&prefix)
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
            
        let mut attestations = Vec::new();
        
        for key in keys {
            let attestation: Attestation = self.storage.get_json(&key)
                .map_err(|e| ReputationError::StorageError(e.to_string()))?;
                
            // Filter out expired attestations
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| ReputationError::Other(e.to_string()))?
                .as_secs();
                
            if let Some(expires_at) = attestation.expires_at {
                if now > expires_at {
                    continue;
                }
            }
            
            attestations.push(attestation);
        }
        
        Ok(attestations)
    }
    
    /// Revoke an attestation
    pub fn revoke_attestation(&self, attestation_id: &str) -> ReputationResult<()> {
        let mut attestation: Attestation = self.storage.get_json(&format!("attestations/{}", attestation_id))
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
            
        attestation.is_revoked = true;
        
        self.storage.put_json(&format!("attestations/{}", attestation_id), &attestation)
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
            
        Ok(())
    }
}

/// Trust graph for calculating indirect trust
pub struct TrustGraph {
    storage: Arc<dyn Storage>,
}

impl TrustGraph {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        TrustGraph { storage }
    }
    
    /// Calculate indirect trust between two nodes
    pub fn calculate_indirect_trust(
        &self,
        source_did: &str,
        target_did: &str,
        max_depth: usize,
        min_trust_threshold: f64,
    ) -> ReputationResult<Option<f64>> {
        let mut graph = HashMap::new();
        
        // Build trust graph
        let prefix = "attestations/att:*:*:*";
        let keys = self.storage.list(prefix)
            .map_err(|e| ReputationError::StorageError(e.to_string()))?;
            
        for key in keys {
            let attestation: Attestation = self.storage.get_json(&key)
                .map_err(|e| ReputationError::StorageError(e.to_string()))?;
                
            if attestation.is_revoked {
                continue;
            }
            
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| ReputationError::Other(e.to_string()))?
                .as_secs();
                
            if let Some(expires_at) = attestation.expires_at {
                if now > expires_at {
                    continue;
                }
            }
            
            // Add edge to graph
            graph.entry(attestation.issuer_did.clone())
                .or_insert_with(HashMap::new)
                .insert(attestation.subject_did.clone(), attestation.score);
        }
        
        // Find trust path
        self.find_trust_path(source_did, target_did, &graph, max_depth, min_trust_threshold)
    }
    
    /// Find a trust path between two nodes
    fn find_trust_path(
        &self,
        source: &str,
        target: &str,
        graph: &HashMap<String, HashMap<String, f64>>,
        max_depth: usize,
        min_threshold: f64,
    ) -> ReputationResult<Option<f64>> {
        if source == target {
            return Ok(Some(1.0));
        }
        
        if max_depth == 0 {
            return Ok(None);
        }
        
        let mut best_score = 0.0;
        
        if let Some(edges) = graph.get(source) {
            for (next, score) in edges {
                if *score < min_threshold {
                    continue;
                }
                
                if let Some(path_score) = self.find_trust_path(next, target, graph, max_depth - 1, min_threshold)? {
                    let total_score = score * path_score;
                    best_score = best_score.max(total_score);
                }
            }
        }
        
        Ok(if best_score > 0.0 { Some(best_score) } else { None })
    }
}

/// Sybil resistance system
pub struct SybilResistance {
    storage: Arc<dyn Storage>,
    attestation_manager: Arc<AttestationManager>,
}

impl SybilResistance {
    pub fn new(storage: Arc<dyn Storage>, attestation_manager: Arc<AttestationManager>) -> Self {
        SybilResistance {
            storage,
            attestation_manager,
        }
    }
    
    /// Check for sybil attack indicators
    pub fn check_sybil_indicators(&self, did: &str) -> ReputationResult<SybilIndicators> {
        let attestations = self.attestation_manager.get_attestations_for_subject(did)?;
        
        let unique_issuers = attestations.iter()
            .map(|a| &a.issuer_did)
            .collect::<HashSet<_>>()
            .len();
            
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| ReputationError::Other(e.to_string()))?
            .as_secs();
            
        let total_age: u64 = attestations.iter()
            .map(|a| now - a.created_at)
            .sum();
            
        let avg_age = if !attestations.is_empty() {
            total_age as f64 / attestations.len() as f64
        } else {
            0.0
        };
        
        let quorum_attestations = attestations.iter()
            .filter(|a| self.attestation_manager.has_reached_quorum(a))
            .count();
            
        let risk_score = Self::calculate_risk_score(
            unique_issuers,
            avg_age,
            quorum_attestations,
            attestations.len(),
        );
        
        Ok(SybilIndicators {
            unique_issuer_count: unique_issuers,
            avg_attestation_age_seconds: avg_age,
            quorum_attestation_count: quorum_attestations,
            attestation_count: attestations.len(),
            risk_score,
        })
    }
    
    /// Calculate risk score for sybil detection
    fn calculate_risk_score(
        unique_issuers: usize,
        avg_age: f64,
        quorum_attestations: usize,
        total_attestations: usize,
    ) -> f64 {
        let issuer_score = if unique_issuers < 3 {
            1.0 - (unique_issuers as f64 / 3.0)
        } else {
            0.0
        };
        
        let age_score = if avg_age < 86400.0 { // Less than 24 hours
            1.0 - (avg_age / 86400.0)
        } else {
            0.0
        };
        
        let quorum_score = if total_attestations > 0 {
            1.0 - (quorum_attestations as f64 / total_attestations as f64)
        } else {
            1.0
        };
        
        (issuer_score + age_score + quorum_score) / 3.0
    }
}

/// Sybil attack indicators
#[derive(Debug, Clone)]
pub struct SybilIndicators {
    pub unique_issuer_count: usize,
    pub avg_attestation_age_seconds: f64,
    pub quorum_attestation_count: usize,
    pub attestation_count: usize,
    pub risk_score: f64, // 0.0-1.0, lower is better
}

/// The main reputation system
pub struct ReputationSystem {
    identity: Arc<Identity>,
    storage: Arc<dyn Storage>,
    crypto: Arc<CryptoUtils>,
    attestation_manager: Arc<AttestationManager>,
    trust_graph: Arc<TrustGraph>,
    sybil_resistance: Arc<SybilResistance>,
}

impl ReputationSystem {
    pub fn new(
        identity: Arc<Identity>,
        storage: Arc<dyn Storage>,
        crypto: Arc<CryptoUtils>,
    ) -> Self {
        let attestation_manager = Arc::new(AttestationManager::new(
            Arc::clone(&identity),
            Arc::clone(&storage),
            Arc::clone(&crypto),
        ));
        
        let trust_graph = Arc::new(TrustGraph::new(Arc::clone(&storage)));
        
        let sybil_resistance = Arc::new(SybilResistance::new(
            Arc::clone(&storage),
            Arc::clone(&attestation_manager),
        ));
        
        ReputationSystem {
            identity,
            storage,
            crypto,
            attestation_manager,
            trust_graph,
            sybil_resistance,
        }
    }
    
    /// Calculate trust score for a member
    pub fn calculate_trust_score(&self, member_did: &str) -> ReputationResult<TrustScore> {
        let start_time = SystemTime::now();
        
        // Get all attestations
        let attestations = self.attestation_manager.get_attestations_for_subject(member_did)?;
        
        // Calculate component scores
        let mut components = HashMap::new();
        
        for attestation_type in &[
            AttestationType::CooperativeVerification,
            AttestationType::MemberVerification,
            AttestationType::TransactionTrust,
            AttestationType::GovernanceQuality,
            AttestationType::ResourceReliability,
            AttestationType::GeneralTrust,
        ] {
            let type_attestations: Vec<_> = attestations.iter()
                .filter(|a| a.attestation_type == *attestation_type)
                .collect();
                
            if !type_attestations.is_empty() {
                let avg_score = type_attestations.iter()
                    .map(|a| a.score)
                    .sum::<f64>() / type_attestations.len() as f64;
                    
                components.insert(
                    format!("{:?}", attestation_type),
                    avg_score,
                );
            }
        }
        
        // Calculate overall score
        let overall_score = if !components.is_empty() {
            components.values().sum::<f64>() / components.len() as f64
        } else {
            0.0
        };
        
        // Check for sybil attacks
        let sybil_indicators = self.sybil_resistance.check_sybil_indicators(member_did)?;
        
        // Calculate confidence level
        let confidence = Self::calculate_confidence_level(
            attestations.len(),
            sybil_indicators.unique_issuer_count,
            sybil_indicators.risk_score,
        );
        
        let calculation_time = start_time.elapsed()
            .map_err(|e| ReputationError::Other(e.to_string()))?
            .as_secs();
        
        Ok(TrustScore {
            overall_score,
            components,
            attestation_count: attestations.len(),
            calculation_time,
            confidence,
        })
    }
    
    /// Calculate confidence level for a trust score
    fn calculate_confidence_level(
        attestation_count: usize,
        unique_issuers: usize,
        sybil_risk: f64,
    ) -> f64 {
        let attestation_score = if attestation_count < 5 {
            attestation_count as f64 / 5.0
        } else {
            1.0
        };
        
        let issuer_score = if unique_issuers < 3 {
            unique_issuers as f64 / 3.0
        } else {
            1.0
        };
        
        let risk_score = 1.0 - sybil_risk;
        
        (attestation_score + issuer_score + risk_score) / 3.0
    }
    
    /// Get the attestation manager
    pub fn attestation_manager(&self) -> Arc<AttestationManager> {
        Arc::clone(&self.attestation_manager)
    }
    
    /// Get the trust graph
    pub fn trust_graph(&self) -> Arc<TrustGraph> {
        Arc::clone(&self.trust_graph)
    }
    
    /// Get the sybil resistance system
    pub fn sybil_resistance(&self) -> Arc<SybilResistance> {
        Arc::clone(&self.sybil_resistance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[test]
    fn test_attestation_creation() {
        // TODO: Implement test
    }
    
    #[test]
    fn test_trust_calculation() {
        // TODO: Implement test
    }
    
    #[test]
    fn test_sybil_detection() {
        // TODO: Implement test
    }
}
