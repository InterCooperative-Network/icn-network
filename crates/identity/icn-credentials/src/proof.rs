//! Cryptographic proof types for verifiable credentials and presentations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Purpose of a cryptographic proof
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ProofPurpose {
    /// Authentication proof
    Authentication,
    
    /// Assertion proof
    AssertionMethod,
    
    /// Agreement proof
    KeyAgreement,
    
    /// Capability invocation
    CapabilityInvocation,
    
    /// Capability delegation
    CapabilityDelegation,
}

/// Type of cryptographic proof
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProofType {
    /// Ed25519 signature
    Ed25519Signature2020,
    
    /// JSON Web Signature
    JsonWebSignature2020,
    
    /// BBS+ signature
    BbsSignature2020,
    
    /// Ethereum signature
    EcdsaSecp256k1Signature2019,
}

/// A cryptographic proof for a credential or presentation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Proof {
    /// The type of this proof
    #[serde(rename = "type")]
    pub type_: ProofType,
    
    /// When this proof was created
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created: DateTime<Utc>,
    
    /// The verification method used to create this proof
    pub verification_method: String,
    
    /// The purpose of this proof
    pub proof_purpose: ProofPurpose,
    
    /// The signature value of this proof (usually base58 encoded)
    pub signature_value: String,
    
    /// The challenge used in this proof, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
    
    /// The domain of this proof, if any
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    
    /// Additional proof-specific properties
    #[serde(flatten)]
    pub properties: HashMap<String, serde_json::Value>,
}

impl Proof {
    /// Create a new proof
    pub fn new(
        type_: ProofType,
        verification_method: String,
        proof_purpose: ProofPurpose,
        signature_value: String,
    ) -> Self {
        Proof {
            type_,
            created: Utc::now(),
            verification_method,
            proof_purpose,
            signature_value,
            challenge: None,
            domain: None,
            properties: HashMap::new(),
        }
    }
    
    /// Set a challenge
    pub fn set_challenge(&mut self, challenge: String) {
        self.challenge = Some(challenge);
    }
    
    /// Set a domain
    pub fn set_domain(&mut self, domain: String) {
        self.domain = Some(domain);
    }
    
    /// Add a property to the proof
    pub fn add_property<T: Into<serde_json::Value>>(&mut self, name: &str, value: T) {
        self.properties.insert(name.to_string(), value.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_proof_creation() {
        let proof = Proof::new(
            ProofType::Ed25519Signature2020,
            "did:icn:test:issuer#key-1".to_string(),
            ProofPurpose::AssertionMethod,
            "z1234567890abcdef".to_string(),
        );
        
        assert!(matches!(proof.type_, ProofType::Ed25519Signature2020));
        assert_eq!(proof.verification_method, "did:icn:test:issuer#key-1");
        assert!(matches!(proof.proof_purpose, ProofPurpose::AssertionMethod));
        assert_eq!(proof.signature_value, "z1234567890abcdef");
        assert!(proof.challenge.is_none());
        assert!(proof.domain.is_none());
    }
    
    #[test]
    fn test_proof_with_challenge_and_domain() {
        let mut proof = Proof::new(
            ProofType::Ed25519Signature2020,
            "did:icn:test:issuer#key-1".to_string(),
            ProofPurpose::Authentication,
            "z1234567890abcdef".to_string(),
        );
        
        proof.set_challenge("abc123".to_string());
        proof.set_domain("icn.coop".to_string());
        
        assert_eq!(proof.challenge, Some("abc123".to_string()));
        assert_eq!(proof.domain, Some("icn.coop".to_string()));
    }
    
    #[test]
    fn test_proof_with_custom_properties() {
        let mut proof = Proof::new(
            ProofType::Ed25519Signature2020,
            "did:icn:test:issuer#key-1".to_string(),
            ProofPurpose::AssertionMethod,
            "z1234567890abcdef".to_string(),
        );
        
        proof.add_property("nonce", "98765");
        proof.add_property("federationId", "test-federation");
        
        assert_eq!(proof.properties.get("nonce").unwrap().as_str().unwrap(), "98765");
        assert_eq!(proof.properties.get("federationId").unwrap().as_str().unwrap(), "test-federation");
    }
} 