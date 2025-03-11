//! Verifiable Credentials implementation for ICN
//!
//! This crate implements the W3C Verifiable Credentials specification for the ICN project,
//! providing credential issuance, verification, and selective disclosure capabilities.

use chrono::{DateTime, Utc};
use icn_common::{Error, Result};
use icn_did::DidDocument;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

pub mod credential;
pub mod presentation;
pub mod schema;
pub mod issuer;
pub mod verifier;
pub mod proof;
pub mod registry;
pub mod selective_disclosure;

// Re-export main types
pub use credential::{Credential, CredentialSubject, CredentialStatus};
pub use presentation::{Presentation, PresentationOptions};
pub use schema::{CredentialSchema, SchemaProperty};
pub use issuer::{Issuer, IssuanceOptions};
pub use verifier::{Verifier, VerificationOptions, VerificationResult};
pub use proof::{Proof, ProofPurpose, ProofType};
pub use registry::{CredentialRegistry, RegistryOptions};
pub use selective_disclosure::{SelectiveDisclosure, RevealOptions};

/// A verifiable credential with standard W3C fields
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiableCredential {
    /// Credential context
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    
    /// Unique identifier for this credential
    pub id: String,
    
    /// Credential types
    #[serde(rename = "type")]
    pub types: Vec<String>,
    
    /// The DID of the issuer
    pub issuer: String,
    
    /// When the credential was issued
    #[serde(with = "chrono::serde::ts_seconds")]
    pub issuance_date: DateTime<Utc>,
    
    /// When the credential expires (if ever)
    #[serde(with = "chrono::serde::ts_seconds_option", skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<DateTime<Utc>>,
    
    /// The subject of the credential
    pub credential_subject: CredentialSubject,
    
    /// The status of the credential (e.g., for revocation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_status: Option<CredentialStatus>,
    
    /// The credential schema
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_schema: Option<CredentialSchema>,
    
    /// Service to refresh the credential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_service: Option<serde_json::Value>,
    
    /// Terms of use for the credential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terms_of_use: Option<serde_json::Value>,
    
    /// The evidence for the credential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<serde_json::Value>,
    
    /// The cryptographic proof
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}

impl VerifiableCredential {
    /// Create a new unissued credential
    pub fn new(
        issuer: &str,
        types: Vec<String>,
        subject: CredentialSubject,
    ) -> Self {
        VerifiableCredential {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://icn.coop/credentials/v1".to_string(),
            ],
            id: format!("urn:uuid:{}", Uuid::new_v4()),
            types: {
                let mut t = vec!["VerifiableCredential".to_string()];
                t.extend(types);
                t
            },
            issuer: issuer.to_string(),
            issuance_date: Utc::now(),
            expiration_date: None,
            credential_subject: subject,
            credential_status: None,
            credential_schema: None,
            refresh_service: None,
            terms_of_use: None,
            evidence: None,
            proof: None,
        }
    }
    
    /// Set the expiration date
    pub fn set_expiration(&mut self, expiration: DateTime<Utc>) {
        self.expiration_date = Some(expiration);
    }
    
    /// Set the credential schema
    pub fn set_schema(&mut self, schema: CredentialSchema) {
        self.credential_schema = Some(schema);
    }
    
    /// Set the credential status
    pub fn set_status(&mut self, status: CredentialStatus) {
        self.credential_status = Some(status);
    }
    
    /// Set the evidence
    pub fn set_evidence(&mut self, evidence: serde_json::Value) {
        self.evidence = Some(evidence);
    }
    
    /// Check if this credential is expired
    pub fn is_expired(&self) -> bool {
        if let Some(expiration) = self.expiration_date {
            Utc::now() > expiration
        } else {
            false
        }
    }
    
    /// Get the credential subject ID if present
    pub fn subject_id(&self) -> Option<&str> {
        self.credential_subject.id.as_deref()
    }
    
    /// Generate canonical form for signing
    pub fn to_canonical_form(&self) -> Result<String> {
        // Create a copy without the proof
        let mut canonical = self.clone();
        canonical.proof = None;
        
        serde_json::to_string(&canonical)
            .map_err(|e| Error::serialization(format!("Failed to canonicalize credential: {}", e)))
    }
    
    /// Verify the credential
    pub fn verify(&self, issuer_did_doc: &DidDocument) -> Result<bool> {
        if self.is_expired() {
            return Ok(false);
        }
        
        let proof = self.proof.as_ref()
            .ok_or_else(|| Error::validation("Credential has no proof"))?;
            
        let verification_method = proof.verification_method.as_str();
        let signature = proof.signature_value.as_str();
        
        // TODO: Get the actual signature from signature_value based on proof type
        // This is a simplified placeholder
        let signature_bytes = bs58::decode(signature)
            .into_vec()
            .map_err(|e| Error::validation(format!("Invalid signature encoding: {}", e)))?;
            
        let message = self.to_canonical_form()?.as_bytes().to_vec();
        
        // Verify using the issuer's DID document
        issuer_did_doc.verify_signature(
            verification_method,
            &message,
            &icn_crypto::Signature::new_from_bytes(signature_bytes),
        )
    }
}

/// A verifiable presentation containing multiple credentials
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerifiablePresentation {
    /// Presentation context
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    
    /// Unique identifier for this presentation
    pub id: String,
    
    /// Presentation types
    #[serde(rename = "type")]
    pub types: Vec<String>,
    
    /// The holder of the presentation
    pub holder: String,
    
    /// Verifiable credentials included in this presentation
    pub verifiable_credential: Vec<VerifiableCredential>,
    
    /// The cryptographic proof
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proof: Option<Proof>,
}

impl VerifiablePresentation {
    /// Create a new verifiable presentation
    pub fn new(holder: &str) -> Self {
        VerifiablePresentation {
            context: vec![
                "https://www.w3.org/2018/credentials/v1".to_string(),
                "https://icn.coop/credentials/v1".to_string(),
            ],
            id: format!("urn:uuid:{}", Uuid::new_v4()),
            types: vec!["VerifiablePresentation".to_string()],
            holder: holder.to_string(),
            verifiable_credential: Vec::new(),
            proof: None,
        }
    }
    
    /// Add a credential to the presentation
    pub fn add_credential(&mut self, credential: VerifiableCredential) {
        self.verifiable_credential.push(credential);
    }
    
    /// Generate canonical form for signing
    pub fn to_canonical_form(&self) -> Result<String> {
        // Create a copy without the proof
        let mut canonical = self.clone();
        canonical.proof = None;
        
        serde_json::to_string(&canonical)
            .map_err(|e| Error::serialization(format!("Failed to canonicalize presentation: {}", e)))
    }
    
    /// Verify the presentation and all included credentials
    pub fn verify(
        &self,
        holder_did_doc: &DidDocument,
        issuer_did_docs: &HashMap<String, DidDocument>,
    ) -> Result<bool> {
        // First verify the presentation proof
        if let Some(proof) = &self.proof {
            let verification_method = proof.verification_method.as_str();
            let signature = proof.signature_value.as_str();
            
            // TODO: Get the actual signature from signature_value based on proof type
            // This is a simplified placeholder
            let signature_bytes = bs58::decode(signature)
                .into_vec()
                .map_err(|e| Error::validation(format!("Invalid signature encoding: {}", e)))?;
                
            let message = self.to_canonical_form()?.as_bytes().to_vec();
            
            // Verify using the holder's DID document
            let presentation_valid = holder_did_doc.verify_signature(
                verification_method,
                &message,
                &icn_crypto::Signature::new_from_bytes(signature_bytes),
            )?;
            
            if !presentation_valid {
                return Ok(false);
            }
        } else {
            return Err(Error::validation("Presentation has no proof"));
        }
        
        // Then verify each credential
        for credential in &self.verifiable_credential {
            let issuer_id = &credential.issuer;
            let issuer_doc = issuer_did_docs.get(issuer_id)
                .ok_or_else(|| Error::not_found(format!("Issuer DID document not found: {}", issuer_id)))?;
                
            if !credential.verify(issuer_doc)? {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_credential_creation() {
        let mut subject = CredentialSubject::new(Some("did:icn:test:123".to_string()));
        subject.add_property("name", "John Doe");
        
        let mut credential = VerifiableCredential::new(
            "did:icn:issuer:456",
            vec!["IdentityCredential".to_string()],
            subject,
        );
        
        assert!(credential.types.contains(&"VerifiableCredential".to_string()));
        assert!(credential.types.contains(&"IdentityCredential".to_string()));
        assert_eq!(credential.issuer, "did:icn:issuer:456");
        assert!(credential.proof.is_none());
        
        let schema = CredentialSchema {
            id: "https://icn.coop/schemas/identity".to_string(),
            type_: "JsonSchema".to_string(),
            properties: HashMap::new(),
        };
        
        credential.set_schema(schema);
        assert!(credential.credential_schema.is_some());
    }
    
    #[test]
    fn test_presentation_creation() {
        let mut subject = CredentialSubject::new(Some("did:icn:test:123".to_string()));
        subject.add_property("name", "John Doe");
        
        let credential = VerifiableCredential::new(
            "did:icn:issuer:456",
            vec!["IdentityCredential".to_string()],
            subject,
        );
        
        let mut presentation = VerifiablePresentation::new("did:icn:test:123");
        presentation.add_credential(credential);
        
        assert_eq!(presentation.holder, "did:icn:test:123");
        assert_eq!(presentation.verifiable_credential.len(), 1);
        assert_eq!(presentation.verifiable_credential[0].issuer, "did:icn:issuer:456");
    }
} 