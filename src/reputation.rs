use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH, Duration};
use serde::{Deserialize, Serialize};
use crate::identity::{Identity, DidDocument};
use crate::crypto::CryptoUtils;
use icn_core::storage::Storage;
// Import directly from the crate
use ed25519_dalek::{Keypair, PublicKey, Signature};

// Reputation system error types
#[derive(Debug)]
pub enum ReputationError {
    InvalidAttestation(String),
    VerificationFailed(String),
    AttestationNotFound(String),
    StorageError(String),
    InvalidScore(String),
    SybilDetected(String),
}

impl fmt::Display for ReputationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReputationError::InvalidAttestation(msg) => write!(f, "Invalid attestation: {}", msg),
            ReputationError::VerificationFailed(msg) => write!(f, "Verification failed: {}", msg),
            ReputationError::AttestationNotFound(msg) => write!(f, "Attestation not found: {}", msg),
            ReputationError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            ReputationError::InvalidScore(msg) => write!(f, "Invalid score: {}", msg),
            ReputationError::SybilDetected(msg) => write!(f, "Sybil attack detected: {}", msg),
        }
    }
}

impl Error for ReputationError {}

// Attestation types
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

// Evidence for attestations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub evidence_type: String,
    pub evidence_id: String,
    pub description: String,
    pub timestamp: u64,
    pub data: Option<serde_json::Value>,
}

// Multi-party signature for attestations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiPartySignature {
    pub signer_did: String,
    pub signature: Vec<u8>,
    pub timestamp: u64,
    pub is_revoked: bool,
}

// Core attestation structure
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

// Trust score with detailed components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    pub overall_score: f64, // Value between 0.0 and 1.0
    pub components: HashMap<String, f64>,
    pub attestation_count: usize,
    pub calculation_time: u64,
    pub confidence: f64, // How confident we are in this score
}

// The main attestation manager
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

    // Create a new attestation with an optional quorum requirement
    pub fn create_attestation(
        &self,
        subject_did: &str,
        attestation_type: AttestationType,
        score: f64,
        claims: serde_json::Value,
        evidence: Vec<Evidence>,
        quorum_threshold: u32,
        expiration_days: Option<u64>,
    ) -> Result<Attestation, Box<dyn Error>> {
        // Validate score range
        if score < 0.0 || score > 1.0 {
            return Err(Box::new(ReputationError::InvalidScore(
                "Score must be between 0.0 and 1.0".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let issuer_did = self.identity.did.clone();
        
        // Generate a unique ID for the attestation
        let id = format!("att:{}:{}:{}", issuer_did, subject_did, now);
        
        // Calculate expiration (no u32 to u64 conversion issue)
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
        let signature = self.identity.sign(signature_data.as_bytes())?;
        
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
        println!("Storing attestation at: attestations/{}", attestation.id);
        match self.storage.put_json(&format!("attestations/{}", attestation.id), &attestation) {
            Ok(_) => println!("Successfully stored attestation"),
            Err(e) => println!("Error storing attestation: {:?}", e),
        }
        
        Ok(attestation)
    }
    
    // Add a signature to an attestation (for multi-party attestations)
    pub fn sign_attestation(
        &self,
        attestation_id: &str,
        signer_did: &str,
        signature: Vec<u8>,
    ) -> Result<Attestation, Box<dyn Error>> {
        // Load the attestation
        let mut attestation: Attestation = self.storage.get_json(&format!("attestations/{}", attestation_id))?;
        
        // Verify that the attestation is not expired
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        if let Some(expires_at) = attestation.expires_at {
            if now > expires_at {
                return Err(Box::new(ReputationError::InvalidAttestation(
                    "Attestation has expired".to_string(),
                )));
            }
        }
        
        // Verify that the attestation is not revoked
        if attestation.is_revoked {
            return Err(Box::new(ReputationError::InvalidAttestation(
                "Attestation has been revoked".to_string(),
            )));
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
        self.storage.put_json(&format!("attestations/{}", attestation.id), &attestation)?;
        
        Ok(attestation)
    }
    
    // Check if an attestation has reached its quorum threshold
    pub fn has_reached_quorum(&self, attestation: &Attestation) -> bool {
        let valid_signatures = attestation.signatures.iter()
            .filter(|sig| !sig.is_revoked)
            .count() as u32;
            
        valid_signatures >= attestation.quorum_threshold
    }
    
    // Get attestations for a subject
    pub fn get_attestations_for_subject(&self, subject_did: &str) -> Result<Vec<Attestation>, Box<dyn Error>> {
        println!("Getting attestations for subject: {}", subject_did);
        
        // In a real implementation, we'd query the storage system more efficiently
        // This is simplified for demonstration purposes
        let attestation_ids = match self.storage.list("attestations/") {
            Ok(ids) => {
                println!("Found {} attestation IDs", ids.len());
                ids
            },
            Err(e) => {
                println!("Error listing attestations: {:?}", e);
                // If the directory doesn't exist, return an empty list
                if let Some(os_error) = e.downcast_ref::<std::io::Error>() {
                    if os_error.kind() == std::io::ErrorKind::NotFound {
                        println!("Attestations directory not found, returning empty list");
                        return Ok(Vec::new());
                    }
                }
                return Err(e);
            }
        };
        
        let mut result = Vec::new();
        
        for id in attestation_ids {
            println!("Reading attestation: {}", id);
            match self.storage.get_json::<Attestation>(&id) {
                Ok(attestation) => {
                    if attestation.subject_did == subject_did && !attestation.is_revoked {
                        println!("Found attestation for subject: {}", subject_did);
                        result.push(attestation);
                    }
                },
                Err(e) => {
                    println!("Error reading attestation {}: {:?}", id, e);
                    // Continue with other attestations
                }
            }
        }
        
        println!("Found {} attestations for subject {}", result.len(), subject_did);
        Ok(result)
    }
    
    // Revoke an attestation
    pub fn revoke_attestation(&self, attestation_id: &str) -> Result<(), Box<dyn Error>> {
        // Load the attestation
        let mut attestation: Attestation = self.storage.get_json(&format!("attestations/{}", attestation_id))?;
        
        // Verify the caller is the issuer
        if attestation.issuer_did != self.identity.did {
            return Err(Box::new(ReputationError::VerificationFailed(
                "Only the issuer can revoke an attestation".to_string(),
            )));
        }
        
        // Mark as revoked
        attestation.is_revoked = true;
        
        // Update in storage
        self.storage.put_json(&format!("attestations/{}", attestation.id), &attestation)?;
        
        Ok(())
    }
}

// Trust graph for indirect trust calculation
pub struct TrustGraph {
    storage: Arc<dyn Storage>,
}

impl TrustGraph {
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        TrustGraph {
            storage,
        }
    }
    
    // Calculate trust between parties that don't have direct attestations
    pub fn calculate_indirect_trust(
        &self,
        source_did: &str,
        target_did: &str,
        max_depth: usize,
        min_trust_threshold: f64,
    ) -> Result<Option<f64>, Box<dyn Error>> {
        // This would be a more sophisticated algorithm in a real implementation
        // For example, using a variation of PageRank or path finding algorithm
        
        // For demonstration, we implement a simplified approach:
        
        // Get all attestations
        let attestation_ids = self.storage.list("attestations/")?;
        
        // Build an adjacency list for the trust graph
        let mut graph: HashMap<String, HashMap<String, f64>> = HashMap::new();
        
        for id in attestation_ids {
            let attestation: Attestation = self.storage.get_json(&format!("attestations/{}", id))?;
            if attestation.is_revoked {
                continue;
            }
            
            if attestation.score < min_trust_threshold {
                continue;
            }
            
            // Add to graph
            graph.entry(attestation.issuer_did.clone())
                .or_insert_with(HashMap::new)
                .insert(attestation.subject_did.clone(), attestation.score);
        }
        
        // Simple path finding with depth limit
        self.find_trust_path(source_did, target_did, &graph, max_depth, min_trust_threshold)
    }
    
    // Helper for path finding
    fn find_trust_path(
        &self,
        source: &str,
        target: &str,
        graph: &HashMap<String, HashMap<String, f64>>,
        max_depth: usize,
        min_threshold: f64,
    ) -> Result<Option<f64>, Box<dyn Error>> {
        if max_depth == 0 {
            return Ok(None);
        }
        
        // Check for direct trust
        if let Some(neighbors) = graph.get(source) {
            if let Some(trust) = neighbors.get(target) {
                return Ok(Some(*trust));
            }
        }
        
        // Check one level of indirection
        if max_depth > 1 {
            let mut best_trust = None;
            
            if let Some(neighbors) = graph.get(source) {
                for (intermediate, trust1) in neighbors {
                    if intermediate == source || intermediate == target {
                        continue;
                    }
                    
                    if let Ok(Some(trust2)) = self.find_trust_path(intermediate, target, graph, max_depth - 1, min_threshold) {
                        // Calculate transitive trust (simplified multiplication)
                        let transitive_trust = trust1 * trust2;
                        
                        if transitive_trust >= min_threshold {
                            // Update best trust path
                            match best_trust {
                                None => best_trust = Some(transitive_trust),
                                Some(current_best) if transitive_trust > current_best => best_trust = Some(transitive_trust),
                                _ => {}
                            }
                        }
                    }
                }
            }
            
            return Ok(best_trust);
        }
        
        Ok(None)
    }
}

// Anti-Sybil mechanisms
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
    
    // Check for potential Sybil attack patterns
    pub fn check_sybil_indicators(&self, did: &str) -> Result<SybilIndicators, Box<dyn Error>> {
        println!("Checking Sybil indicators for: {}", did);
        
        // Create a default set of indicators if no attestations are found
        let default_indicators = SybilIndicators {
            unique_issuer_count: 0,
            avg_attestation_age_seconds: 0.0,
            quorum_attestation_count: 0,
            attestation_count: 0,
            risk_score: 0.5, // Neutral risk score
        };
        
        // Try to get attestations, but don't fail if there are none
        let attestations = match self.attestation_manager.get_attestations_for_subject(did) {
            Ok(atts) => {
                println!("Found {} attestations for Sybil check", atts.len());
                atts
            },
            Err(e) => {
                println!("Error getting attestations for Sybil check: {:?}", e);
                return Ok(default_indicators);
            }
        };
        
        if attestations.is_empty() {
            println!("No attestations found for Sybil check, using default indicators");
            return Ok(default_indicators);
        }
        
        // 1. Check for attestation diversity
        let unique_issuers: HashSet<String> = attestations.iter()
            .map(|att| att.issuer_did.clone())
            .collect();
            
        // 2. Check for attestation age distribution
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        let avg_age = if attestations.is_empty() {
            0.0
        } else {
            let total_age: u64 = attestations.iter()
                .map(|att| now - att.created_at)
                .sum();
            total_age as f64 / attestations.len() as f64
        };
        
        // 3. Check quorum attestations
        let quorum_attestations = attestations.iter()
            .filter(|att| self.attestation_manager.has_reached_quorum(att))
            .count();
            
        let indicators = SybilIndicators {
            unique_issuer_count: unique_issuers.len(),
            avg_attestation_age_seconds: avg_age,
            quorum_attestation_count: quorum_attestations,
            attestation_count: attestations.len(),
            risk_score: Self::calculate_risk_score(
                unique_issuers.len(),
                avg_age,
                quorum_attestations,
                attestations.len(),
            ),
        };
        
        println!("Sybil indicators: {:?}", indicators);
        Ok(indicators)
    }
    
    // Calculate Sybil risk score (lower is better)
    fn calculate_risk_score(
        unique_issuers: usize,
        avg_age: f64,
        quorum_attestations: usize,
        total_attestations: usize,
    ) -> f64 {
        // These weights would be tuned based on empirical data
        let issuer_weight = 0.4;
        let age_weight = 0.3;
        let quorum_weight = 0.3;
        
        // Calculate normalized factors (higher values are better)
        let issuer_factor = if total_attestations == 0 {
            0.0
        } else {
            (unique_issuers as f64 / total_attestations as f64).min(1.0)
        };
        
        // Age factor - older attestations are better (up to 90 days)
        let max_age = 90.0 * 24.0 * 60.0 * 60.0; // 90 days in seconds
        let age_factor = (avg_age / max_age).min(1.0);
        
        // Quorum factor - more quorum attestations are better
        let quorum_factor = if total_attestations == 0 {
            0.0
        } else {
            (quorum_attestations as f64 / total_attestations as f64)
        };
        
        // Calculate risk score (lower is better)
        1.0 - (
            issuer_factor * issuer_weight +
            age_factor * age_weight +
            quorum_factor * quorum_weight
        )
    }
}

// Sybil risk indicators
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SybilIndicators {
    pub unique_issuer_count: usize,
    pub avg_attestation_age_seconds: f64,
    pub quorum_attestation_count: usize,
    pub attestation_count: usize,
    pub risk_score: f64, // 0.0-1.0, lower is better
}

// Main reputation system
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
            identity.clone(),
            storage.clone(),
            crypto.clone(),
        ));
        
        let trust_graph = Arc::new(TrustGraph::new(
            storage.clone(),
        ));
        
        let sybil_resistance = Arc::new(SybilResistance::new(
            storage.clone(),
            attestation_manager.clone(),
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
    
    // Calculate comprehensive trust score for a DID
    pub fn calculate_trust_score(&self, member_did: &str) -> Result<TrustScore, Box<dyn Error>> {
        println!("Calculating trust score for member: {}", member_did);
        
        // Get all attestations for this member
        let attestations_path = "attestations";
        println!("Looking for attestations in path: {}", attestations_path);
        
        let attestation_files = match self.storage.list(attestations_path) {
            Ok(files) => {
                println!("Found {} attestation files", files.len());
                files
            },
            Err(e) => {
                println!("Error listing attestation files: {:?}", e);
                return Err(e);
            }
        };
        
        let mut attestations = Vec::new();
        for file in attestation_files {
            println!("Reading attestation file: {}", file);
            match self.storage.get_json::<Attestation>(&file) {
                Ok(attestation) => {
                    if attestation.subject_did == member_did {
                        attestations.push(attestation);
                    }
                },
                Err(e) => {
                    println!("Error reading attestation file {}: {:?}", file, e);
                    // Continue with other files
                }
            }
        }
        
        println!("Found {} attestations for member {}", attestations.len(), member_did);
        
        // Check for Sybil indicators
        let sybil_indicators = self.sybil_resistance.check_sybil_indicators(member_did)?;
        
        // Initialize component scores
        let mut components = HashMap::new();
        
        // Calculate attestation-based score
        let mut total_score = 0.0;
        let mut total_weight = 0.0;
        
        for attestation in &attestations {
            // Skip attestations that haven't reached quorum
            if !self.attestation_manager.has_reached_quorum(attestation) {
                continue;
            }
            
            // Calculate age-based weight (newer attestations have more weight)
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs();
                
            let age_seconds = now - attestation.created_at;
            let age_days = age_seconds as f64 / (24.0 * 60.0 * 60.0);
            
            // Weight decays by half every 90 days
            let weight = 1.0 / (1.0 + age_days / 90.0);
            
            // Add to total
            total_score += attestation.score * weight;
            total_weight += weight;
            
            // Add to component score based on attestation type
            let component_key = format!("{:?}", attestation.attestation_type);
            let component_score = components.entry(component_key).or_insert(0.0);
            *component_score += attestation.score * weight;
        }
        
        // Normalize component scores
        if total_weight > 0.0 {
            for score in components.values_mut() {
                *score /= total_weight;
            }
        }
        
        // Calculate overall score
        let attestation_score = if total_weight > 0.0 {
            total_score / total_weight
        } else {
            0.0
        };
        
        // Apply Sybil risk adjustment (reduce score based on risk)
        let adjusted_score = attestation_score * (1.0 - sybil_indicators.risk_score * 0.5);
        
        // Add Sybil components
        components.insert("sybil_risk".to_string(), sybil_indicators.risk_score);
        
        // Create final trust score
        let trust_score = TrustScore {
            overall_score: adjusted_score,
            components,
            attestation_count: attestations.len(),
            calculation_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)?
                .as_secs(),
            confidence: Self::calculate_confidence_level(
                attestations.len(),
                sybil_indicators.unique_issuer_count,
                sybil_indicators.risk_score,
            ),
        };
        
        Ok(trust_score)
    }
    
    // Helper to calculate confidence in the trust score
    fn calculate_confidence_level(
        attestation_count: usize,
        unique_issuers: usize,
        sybil_risk: f64,
    ) -> f64 {
        // This is a simplified model that would be tuned based on network data
        
        // More attestations increase confidence (up to a point)
        let count_factor = (attestation_count as f64 / 10.0).min(1.0);
        
        // More unique issuers increase confidence
        let issuer_factor = if attestation_count == 0 {
            0.0
        } else {
            (unique_issuers as f64 / attestation_count as f64).min(1.0)
        };
        
        // Lower Sybil risk increases confidence
        let risk_factor = 1.0 - sybil_risk;
        
        // Combine factors (weighted average)
        let count_weight = 0.2;
        let issuer_weight = 0.5;
        let risk_weight = 0.3;
        
        count_factor * count_weight +
        issuer_factor * issuer_weight +
        risk_factor * risk_weight
    }
    
    // Get the attestation manager
    pub fn attestation_manager(&self) -> Arc<AttestationManager> {
        self.attestation_manager.clone()
    }
    
    // Get the trust graph
    pub fn trust_graph(&self) -> Arc<TrustGraph> {
        self.trust_graph.clone()
    }
    
    // Get the Sybil resistance module
    pub fn sybil_resistance(&self) -> Arc<SybilResistance> {
        self.sybil_resistance.clone()
    }
} 