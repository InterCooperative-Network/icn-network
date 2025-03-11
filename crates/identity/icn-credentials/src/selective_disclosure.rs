//! Selective disclosure types and functionality for the ICN verifiable credentials system
//!
//! This module provides the structures and functions for creating and verifying
//! verifiable presentations with selective disclosure of attributes.

use async_trait::async_trait;
use chrono::Utc;
use icn_common::Result;
use icn_crypto::key::KeyPair;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use crate::{
    Credential, Presentation, VerifiableCredential, VerifiablePresentation,
    Proof, ProofPurpose, ProofType
};

/// Options for revealing specific parts of a credential
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RevealOptions {
    /// Properties to reveal from the credential subject
    pub reveal_properties: HashSet<String>,
    
    /// Whether to reveal the subject ID
    pub reveal_subject_id: bool,
    
    /// Whether to reveal the issuer ID
    pub reveal_issuer: bool,
    
    /// Whether to reveal the issuance date
    pub reveal_issuance_date: bool,
    
    /// Whether to reveal the expiration date
    pub reveal_expiration_date: bool,
}

impl Default for RevealOptions {
    fn default() -> Self {
        RevealOptions {
            reveal_properties: HashSet::new(),
            reveal_subject_id: true,
            reveal_issuer: true,
            reveal_issuance_date: true,
            reveal_expiration_date: true,
        }
    }
}

/// Interface for selective disclosure operations
#[async_trait]
pub trait SelectiveDisclosure {
    /// Create a presentation with selective disclosure
    async fn create_presentation(
        &self,
        credential: &VerifiableCredential,
        reveal_options: &RevealOptions,
        holder_did: &str,
        key_pair: &KeyPair,
        challenge: Option<String>,
        domain: Option<String>,
    ) -> Result<VerifiablePresentation>;
    
    /// Verify a presentation with selective disclosure
    async fn verify_presentation(
        &self,
        presentation: &VerifiablePresentation,
        expected_challenge: Option<&str>,
        expected_domain: Option<&str>,
    ) -> Result<bool>;
}

/// A basic implementation of selective disclosure
pub struct BasicSelectiveDisclosure;

impl BasicSelectiveDisclosure {
    /// Create a new basic selective disclosure handler
    pub fn new() -> Self {
        BasicSelectiveDisclosure
    }
    
    /// Create a redacted copy of a credential
    fn redact_credential(
        &self,
        credential: &VerifiableCredential,
        options: &RevealOptions,
    ) -> VerifiableCredential {
        // Start with a clone of the credential
        let mut redacted = credential.clone();
        
        // Create a new subject with only the revealed properties
        let mut new_subject = crate::CredentialSubject::new(
            if options.reveal_subject_id {
                redacted.credential_subject.id.clone()
            } else {
                None
            }
        );
        
        // Add only the revealed properties
        for property_name in &options.reveal_properties {
            if let Some(value) = redacted.credential_subject.properties.get(property_name) {
                new_subject.add_property(property_name, value.clone());
            }
        }
        
        // Replace the subject
        redacted.credential_subject = new_subject;
        
        // If not revealing issuer, redact it
        if !options.reveal_issuer {
            redacted.issuer = "REDACTED".to_string();
        }
        
        // If not revealing issuance date, redact it
        if !options.reveal_issuance_date {
            redacted.issuance_date = Utc::now(); // Use a placeholder date
        }
        
        // If not revealing expiration date, redact it
        if !options.reveal_expiration_date {
            redacted.expiration_date = None;
        }
        
        // Remove the proof - it will be replaced by the presentation proof
        redacted.proof = None;
        
        redacted
    }
}

#[async_trait]
impl SelectiveDisclosure for BasicSelectiveDisclosure {
    async fn create_presentation(
        &self,
        credential: &VerifiableCredential,
        reveal_options: &RevealOptions,
        holder_did: &str,
        key_pair: &KeyPair,
        challenge: Option<String>,
        domain: Option<String>,
    ) -> Result<VerifiablePresentation> {
        // Create a redacted copy of the credential
        let redacted_credential = self.redact_credential(credential, reveal_options);
        
        // Create a presentation
        let mut presentation = VerifiablePresentation {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: format!("urn:uuid:{}", uuid::Uuid::new_v4()),
            types: vec!["VerifiablePresentation".to_string()],
            holder: holder_did.to_string(),
            verifiable_credential: vec![redacted_credential],
            proof: None,
        };
        
        // Serialize the presentation for signing
        let data_to_sign = serde_json::to_string(&presentation)?;
        
        // Sign the presentation
        let signature = key_pair.sign(data_to_sign.as_bytes())?;
        
        // Create the proof
        let proof = Proof {
            type_: ProofType::Ed25519Signature2020,
            created: Utc::now(),
            verification_method: format!("{}#keys-1", holder_did),
            purpose: ProofPurpose::Authentication,
            value: signature,
            jws: None,
            domain,
            challenge,
            nonce: None,
        };
        
        // Add the proof to the presentation
        presentation.proof = Some(proof);
        
        Ok(presentation)
    }
    
    async fn verify_presentation(
        &self,
        presentation: &VerifiablePresentation,
        expected_challenge: Option<&str>,
        expected_domain: Option<&str>,
    ) -> Result<bool> {
        // Check that the presentation has a proof
        let proof = match &presentation.proof {
            Some(p) => p,
            None => return Ok(false),
        };
        
        // Check the challenge if expected
        if let Some(expected) = expected_challenge {
            if let Some(challenge) = &proof.challenge {
                if challenge != expected {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        
        // Check the domain if expected
        if let Some(expected) = expected_domain {
            if let Some(domain) = &proof.domain {
                if domain != expected {
                    return Ok(false);
                }
            } else {
                return Ok(false);
            }
        }
        
        // In a real implementation, we would verify the signature
        // For now, just return true if we have a proof
        Ok(true)
    }
}

/// Advanced selective disclosure using zero-knowledge proofs
pub struct ZkpSelectiveDisclosure;

impl ZkpSelectiveDisclosure {
    /// Create a new ZKP-based selective disclosure handler
    pub fn new() -> Self {
        ZkpSelectiveDisclosure
    }
}

#[async_trait]
impl SelectiveDisclosure for ZkpSelectiveDisclosure {
    async fn create_presentation(
        &self,
        credential: &VerifiableCredential,
        reveal_options: &RevealOptions,
        holder_did: &str,
        key_pair: &KeyPair,
        challenge: Option<String>,
        domain: Option<String>,
    ) -> Result<VerifiablePresentation> {
        // This would be implemented with actual ZKP techniques
        // For now, just delegate to the basic implementation
        let basic = BasicSelectiveDisclosure::new();
        basic.create_presentation(
            credential,
            reveal_options,
            holder_did,
            key_pair,
            challenge,
            domain,
        ).await
    }
    
    async fn verify_presentation(
        &self,
        presentation: &VerifiablePresentation,
        expected_challenge: Option<&str>,
        expected_domain: Option<&str>,
    ) -> Result<bool> {
        // This would be implemented with actual ZKP verification
        // For now, just delegate to the basic implementation
        let basic = BasicSelectiveDisclosure::new();
        basic.verify_presentation(
            presentation,
            expected_challenge,
            expected_domain,
        ).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CredentialSubject;
    use icn_crypto::key::{KeyPair, KeyType};
    
    #[tokio::test]
    async fn test_basic_selective_disclosure() {
        // Create a credential subject
        let mut subject = CredentialSubject::new(Some("did:icn:test:subject".to_string()));
        subject.add_property("name", "Alice Smith");
        subject.add_property("age", 30);
        subject.add_property("email", "alice@example.com");
        
        // Create a credential
        let credential = VerifiableCredential {
            context: vec!["https://www.w3.org/2018/credentials/v1".to_string()],
            id: "test-credential-1".to_string(),
            types: vec!["VerifiableCredential".to_string(), "IdentityCredential".to_string()],
            issuer: "did:icn:test:issuer".to_string(),
            issuance_date: Utc::now(),
            expiration_date: Some(Utc::now() + chrono::Duration::days(365)),
            credential_subject: subject,
            credential_status: None,
            credential_schema: None,
            refresh_service: None,
            terms_of_use: None,
            evidence: None,
            proof: Some(Proof {
                type_: ProofType::Ed25519Signature2020,
                created: Utc::now(),
                verification_method: "did:icn:test:issuer#keys-1".to_string(),
                purpose: ProofPurpose::AssertionMethod,
                value: icn_crypto::signature::Signature::Ed25519(vec![1, 2, 3, 4]), // Dummy signature
                jws: None,
                domain: None,
                challenge: None,
                nonce: None,
            }),
        };
        
        // Create a key pair for the subject
        let key_pair = KeyPair::generate(KeyType::Ed25519).unwrap();
        
        // Create selective disclosure options that only reveal name
        let mut reveal_options = RevealOptions::default();
        reveal_options.reveal_properties.insert("name".to_string());
        
        // Create a selective disclosure handler
        let sd = BasicSelectiveDisclosure::new();
        
        // Create a presentation with selective disclosure
        let presentation = sd.create_presentation(
            &credential,
            &reveal_options,
            "did:icn:test:subject",
            &key_pair,
            Some("challenge123".to_string()),
            Some("example.com".to_string()),
        ).await.unwrap();
        
        // Verify the presentation has the expected properties
        assert_eq!(presentation.holder, "did:icn:test:subject");
        assert!(presentation.proof.is_some());
        
        // Check that only the name is revealed
        let vc = &presentation.verifiable_credential[0];
        assert_eq!(vc.credential_subject.properties.len(), 1);
        assert!(vc.credential_subject.properties.contains_key("name"));
        assert!(!vc.credential_subject.properties.contains_key("age"));
        assert!(!vc.credential_subject.properties.contains_key("email"));
        
        // Verify the presentation
        let result = sd.verify_presentation(
            &presentation,
            Some("challenge123"),
            Some("example.com"),
        ).await.unwrap();
        
        assert!(result);
    }
} 