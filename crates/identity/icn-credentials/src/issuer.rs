//! Issuer types and functionality for the ICN verifiable credentials system
//!
//! This module provides the structures and functions for issuers to create
//! and sign verifiable credentials.

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use icn_common::Result;
use icn_did::DidDocument;
use icn_crypto::signature::Signature;
use icn_crypto::key::{KeyPair, KeyType};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Credential, CredentialSubject, Proof, ProofPurpose, ProofType, VerifiableCredential};

/// Options for creating verifiable credentials
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IssuanceOptions {
    /// When the credential expires (if it has an expiration)
    pub expires: Option<DateTime<Utc>>,
    
    /// The specific verification method to use
    pub verification_method: Option<String>,
    
    /// The proof type to use for the credential
    pub proof_type: ProofType,
    
    /// The proof purpose to use for the credential
    pub proof_purpose: ProofPurpose,
    
    /// Any additional options specific to the credential type
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_options: Option<serde_json::Value>,
}

impl Default for IssuanceOptions {
    fn default() -> Self {
        IssuanceOptions {
            expires: Some(Utc::now() + Duration::days(365)),
            verification_method: None,
            proof_type: ProofType::Ed25519Signature2020,
            proof_purpose: ProofPurpose::AssertionMethod,
            additional_options: None,
        }
    }
}

/// Issuer interface for creating verifiable credentials
#[async_trait]
pub trait Issuer {
    /// Issue a new verifiable credential
    async fn issue_credential(
        &self,
        id: Option<String>,
        types: Vec<String>,
        subject: CredentialSubject,
        options: IssuanceOptions,
    ) -> Result<VerifiableCredential>;
    
    /// Revoke a previously issued credential
    async fn revoke_credential(&self, credential_id: &str) -> Result<bool>;
    
    /// Get the DID of this issuer
    fn did(&self) -> &str;
    
    /// Get the DID document of this issuer
    async fn did_document(&self) -> Result<DidDocument>;
}

/// A basic implementation of the Issuer trait
pub struct BasicIssuer {
    /// The DID of this issuer
    did: String,
    
    /// The key pair used for signing credentials
    key_pair: KeyPair,
    
    /// The DID document of this issuer
    did_document: DidDocument,
}

impl BasicIssuer {
    /// Create a new basic issuer
    pub fn new(did: String, key_pair: KeyPair, did_document: DidDocument) -> Self {
        BasicIssuer {
            did,
            key_pair,
            did_document,
        }
    }
    
    /// Generate a new issuer with a fresh key pair
    pub fn generate(did_prefix: &str) -> Result<Self> {
        // Generate a new key pair
        let key_pair = KeyPair::generate(KeyType::Ed25519)?;
        let key_id = format!("{}#keys-1", did_prefix);
        
        // Create a DID document
        // In a real implementation, this would be more sophisticated
        let did_document = DidDocument::new(did_prefix.to_string());
        
        Ok(BasicIssuer {
            did: did_prefix.to_string(),
            key_pair,
            did_document,
        })
    }
}

#[async_trait]
impl Issuer for BasicIssuer {
    async fn issue_credential(
        &self,
        id: Option<String>,
        types: Vec<String>,
        subject: CredentialSubject,
        options: IssuanceOptions,
    ) -> Result<VerifiableCredential> {
        // Generate a credential ID if none provided
        let credential_id = id.unwrap_or_else(|| format!("urn:uuid:{}", Uuid::new_v4()));
        
        // Get the verification method to use
        let verification_method = options.verification_method
            .unwrap_or_else(|| format!("{}#keys-1", self.did));
        
        // Create the credential
        let mut credential = VerifiableCredential {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://w3id.org/security/suites/ed25519-2020/v1".to_string(),
            ],
            id: credential_id,
            types: vec!["VerifiableCredential".to_string()]
                .into_iter()
                .chain(types.into_iter())
                .collect(),
            issuer: self.did.clone(),
            issuance_date: Utc::now(),
            expiration_date: options.expires,
            credential_subject: subject,
            credential_status: None,
            credential_schema: None,
            refresh_service: None,
            terms_of_use: None,
            evidence: None,
            proof: None,
        };
        
        // Create the proof
        let data_to_sign = serde_json::to_string(&credential)?;
        let signature = self.key_pair.sign(data_to_sign.as_bytes())?;
        
        // Add the proof to the credential
        let proof = Proof {
            type_: options.proof_type,
            created: Utc::now(),
            verification_method,
            purpose: options.proof_purpose,
            value: signature,
            jws: None,
            domain: None,
            nonce: None,
        };
        
        credential.proof = Some(proof);
        
        Ok(credential)
    }
    
    async fn revoke_credential(&self, credential_id: &str) -> Result<bool> {
        // In a real implementation, this would update a revocation registry
        // For now, just return success
        Ok(true)
    }
    
    fn did(&self) -> &str {
        &self.did
    }
    
    async fn did_document(&self) -> Result<DidDocument> {
        Ok(self.did_document.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_issuer_issue_credential() {
        let issuer = BasicIssuer::generate("did:icn:test:issuer").unwrap();
        
        let mut subject = CredentialSubject::new(Some("did:icn:test:subject".to_string()));
        subject.add_property("name", "Alice");
        subject.add_property("membershipLevel", "Gold");
        
        let options = IssuanceOptions::default();
        
        let credential = issuer.issue_credential(
            None,
            vec!["MembershipCredential".to_string()],
            subject,
            options,
        ).await.unwrap();
        
        assert!(credential.id.starts_with("urn:uuid:"));
        assert_eq!(credential.issuer, "did:icn:test:issuer");
        assert!(credential.types.contains(&"MembershipCredential".to_string()));
        assert!(credential.proof.is_some());
        
        let subject = &credential.credential_subject;
        assert_eq!(subject.id.as_ref().unwrap(), "did:icn:test:subject");
        assert_eq!(subject.get_property("name").unwrap().as_str().unwrap(), "Alice");
        assert_eq!(subject.get_property("membershipLevel").unwrap().as_str().unwrap(), "Gold");
    }
} 