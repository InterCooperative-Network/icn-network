use crate::did::DidDocument;
use crate::error::Error;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Types of predicates that can be used in zero-knowledge proofs
pub enum PredicateType {
    /// Equal to a value
    Equal,
    /// Not equal to a value
    NotEqual,
    /// Greater than a value
    GreaterThan,
    /// Greater than or equal to a value
    GreaterThanOrEqual,
    /// Less than a value
    LessThan,
    /// Less than or equal to a value
    LessThanOrEqual,
    /// In a set of values
    InSet,
    /// Not in a set of values
    NotInSet,
}

/// Range constraint types for zero-knowledge proofs
pub enum RangeConstraint {
    /// Value must be positive
    Positive,
    /// Sender must have sufficient balance
    SufficientBalance,
    /// Value must be within a specific range
    InRange(u64, u64),
}

/// Request for a zero-knowledge proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofRequest {
    /// Unique identifier for the request
    pub id: String,
    /// DID of the verifier
    pub verifier_did: String,
    /// Requested attributes to prove
    pub requested_attributes: HashMap<String, AttributePredicate>,
    /// Requested predicates to satisfy
    pub requested_predicates: Vec<Predicate>,
    /// Nonce to prevent replay attacks
    pub nonce: String,
}

/// Attribute predicate for a zero-knowledge proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AttributePredicate {
    /// Attribute name
    pub name: String,
    /// Predicate type
    pub predicate_type: String,
    /// Predicate value
    pub value: serde_json::Value,
}

/// Predicate for a zero-knowledge proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Predicate {
    /// Predicate name
    pub name: String,
    /// Predicate type
    pub predicate_type: String,
    /// Predicate value
    pub value: serde_json::Value,
}

/// Response containing a zero-knowledge proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProofResponse {
    /// ID of the proof request
    pub request_id: String,
    /// DID of the prover
    pub prover_did: String,
    /// The proof itself
    pub proof: Proof,
    /// Nonce from the request
    pub nonce: String,
}

/// A zero-knowledge proof
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    /// The proof data
    pub proof_data: Vec<u8>,
    /// Cryptographic commitments
    pub commitments: Vec<Commitment>,
    /// Revealed attributes (if any)
    pub revealed_attributes: HashMap<String, String>,
}

/// A cryptographic commitment
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Commitment {
    /// Name of the attribute
    pub attribute: String,
    /// Commitment value
    pub value: Vec<u8>,
}

/// Interface for generating and verifying zero-knowledge proofs
#[async_trait]
pub trait ZkpProvider: Send + Sync {
    /// Generate a proof for a request
    async fn generate_proof(&self, request: &ProofRequest, did: &str) -> Result<ProofResponse, Error>;
    
    /// Verify a proof
    async fn verify_proof(&self, proof: &ProofResponse) -> Result<bool, Error>;
    
    /// Generate a proof for a transaction without revealing the amount
    async fn generate_transaction_proof(
        &self,
        transaction_template: &serde_json::Value,
        amount: u64,
        constraints: &[RangeConstraint],
    ) -> Result<Proof, Error>;
    
    /// Generate a proof of eligibility for voting without revealing identity
    async fn generate_eligibility_proof(
        &self,
        voter_did: &str,
        proposal_id: &str,
    ) -> Result<Proof, Error>;
}

/// Manager for zero-knowledge proofs
pub struct ZkpManager {
    provider: Box<dyn ZkpProvider>,
    proof_cache: RwLock<HashMap<String, ProofResponse>>,
}

impl ZkpManager {
    /// Create a new ZKP manager with the specified provider
    pub fn new(provider: Box<dyn ZkpProvider>) -> Self {
        Self {
            provider,
            proof_cache: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create a proof request
    pub fn create_request() -> ProofRequestBuilder {
        ProofRequestBuilder::new()
    }
    
    /// Request a proof from a subject
    pub async fn request_proof(&self, subject_did: &str, request: &ProofRequest) -> Result<ProofResponse, Error> {
        self.provider.generate_proof(request, subject_did).await
    }
    
    /// Verify a proof
    pub async fn verify_proof(&self, proof: &ProofResponse) -> Result<bool, Error> {
        self.provider.verify_proof(proof).await
    }
    
    /// Generate a transaction proof
    pub async fn generate_transaction_proof(
        &self,
        transaction_template: &serde_json::Value,
        amount: u64,
        constraints: &[RangeConstraint],
    ) -> Result<Proof, Error> {
        self.provider.generate_transaction_proof(transaction_template, amount, constraints).await
    }
    
    /// Generate an eligibility proof for voting
    pub async fn generate_eligibility_proof(
        &self,
        voter_did: &str,
        proposal_id: &str,
    ) -> Result<Proof, Error> {
        self.provider.generate_eligibility_proof(voter_did, proposal_id).await
    }
}

/// Builder for proof requests
pub struct ProofRequestBuilder {
    id: String,
    verifier_did: String,
    requested_attributes: HashMap<String, AttributePredicate>,
    requested_predicates: Vec<Predicate>,
    nonce: String,
}

impl ProofRequestBuilder {
    /// Create a new proof request builder
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            verifier_did: String::new(),
            requested_attributes: HashMap::new(),
            requested_predicates: Vec::new(),
            nonce: uuid::Uuid::new_v4().to_string(),
        }
    }
    
    /// Set the verifier DID
    pub fn verifier(mut self, did: &str) -> Self {
        self.verifier_did = did.to_string();
        self
    }
    
    /// Add an attribute to the request
    pub fn attribute(mut self, name: &str, value: &str) -> Self {
        self.requested_attributes.insert(
            name.to_string(),
            AttributePredicate {
                name: name.to_string(),
                predicate_type: "equal".to_string(),
                value: serde_json::Value::String(value.to_string()),
            },
        );
        self
    }
    
    /// Add an attribute predicate to the request
    pub fn attribute_predicate(mut self, name: &str, predicate_type: PredicateType, value: impl Into<serde_json::Value>) -> Self {
        let predicate_type_str = match predicate_type {
            PredicateType::Equal => "equal",
            PredicateType::NotEqual => "notEqual",
            PredicateType::GreaterThan => "greaterThan",
            PredicateType::GreaterThanOrEqual => "greaterThanOrEqual",
            PredicateType::LessThan => "lessThan",
            PredicateType::LessThanOrEqual => "lessThanOrEqual",
            PredicateType::InSet => "inSet",
            PredicateType::NotInSet => "notInSet",
        };
        
        self.requested_attributes.insert(
            name.to_string(),
            AttributePredicate {
                name: name.to_string(),
                predicate_type: predicate_type_str.to_string(),
                value: value.into(),
            },
        );
        self
    }
    
    /// Build the proof request
    pub fn build(self) -> Result<ProofRequest, Error> {
        if self.verifier_did.is_empty() {
            return Err(Error::InvalidInput("Verifier DID must be specified".into()));
        }
        
        Ok(ProofRequest {
            id: self.id,
            verifier_did: self.verifier_did,
            requested_attributes: self.requested_attributes,
            requested_predicates: self.requested_predicates,
            nonce: self.nonce,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proof_request_builder() {
        let request = ProofRequestBuilder::new()
            .verifier("did:icn:test:verifier")
            .attribute("name", "Test")
            .attribute_predicate("age", PredicateType::GreaterThanOrEqual, 18)
            .build()
            .unwrap();
        
        assert_eq!(request.verifier_did, "did:icn:test:verifier");
        assert_eq!(request.requested_attributes.len(), 2);
        assert!(request.requested_attributes.contains_key("name"));
        assert!(request.requested_attributes.contains_key("age"));
    }
} 