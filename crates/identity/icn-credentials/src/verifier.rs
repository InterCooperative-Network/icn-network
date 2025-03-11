//! Verifier types and functionality for the ICN verifiable credentials system
//!
//! This module provides the structures and functions for verifying credentials
//! and presentations.

use async_trait::async_trait;
use chrono::Utc;
use icn_common::{Error, Result};
use icn_did::{Did, DidDocument, DidResolver};
use icn_crypto::key::PublicKey;
use serde::{Deserialize, Serialize};

use crate::{
    Credential, Presentation, VerifiableCredential, VerifiablePresentation,
    Proof, ProofPurpose, ProofType
};

/// Result of verifying a credential or presentation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether verification succeeded
    pub verified: bool,
    
    /// Specific verification checks that were performed
    pub checks: Vec<VerificationCheck>,
    
    /// Error messages if verification failed
    pub errors: Vec<String>,
}

impl VerificationResult {
    /// Create a successful verification result
    pub fn success(checks: Vec<VerificationCheck>) -> Self {
        VerificationResult {
            verified: true,
            checks,
            errors: Vec::new(),
        }
    }
    
    /// Create a failed verification result
    pub fn failure(errors: Vec<String>) -> Self {
        VerificationResult {
            verified: false,
            checks: Vec::new(),
            errors,
        }
    }
    
    /// Add a successful check to the result
    pub fn add_check(&mut self, check: VerificationCheck) {
        self.checks.push(check);
    }
    
    /// Add an error to the result and mark as failed
    pub fn add_error(&mut self, error: String) {
        self.verified = false;
        self.errors.push(error);
    }
}

/// Type of verification check performed
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum VerificationCheck {
    /// Check that the signature is valid
    SignatureValid,
    
    /// Check that the credential is not expired
    NotExpired,
    
    /// Check that the credential has not been revoked
    NotRevoked,
    
    /// Check that the issuer is trusted
    IssuerTrusted,
    
    /// Check that the credential schema is valid
    SchemaValid,
    
    /// Custom verification check
    Custom(String),
}

/// Options for verifying a credential or presentation
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationOptions {
    /// Whether to verify the credential signature
    pub verify_signature: bool,
    
    /// Whether to check if the credential is expired
    pub verify_expiration: bool,
    
    /// Whether to check if the credential has been revoked
    pub verify_revocation: bool,
    
    /// Whether to verify the issuer is trusted
    pub verify_issuer: bool,
    
    /// Whether to verify the credential schema
    pub verify_schema: bool,
    
    /// Challenge for presentation verification
    pub challenge: Option<String>,
    
    /// Domain for presentation verification
    pub domain: Option<String>,
    
    /// Acceptable purposes for the proof
    pub accepted_purposes: Vec<ProofPurpose>,
}

impl Default for VerificationOptions {
    fn default() -> Self {
        VerificationOptions {
            verify_signature: true,
            verify_expiration: true,
            verify_revocation: true,
            verify_issuer: true,
            verify_schema: true,
            challenge: None,
            domain: None,
            accepted_purposes: vec![ProofPurpose::AssertionMethod],
        }
    }
}

/// Verifier interface for verifying credentials and presentations
#[async_trait]
pub trait Verifier {
    /// Verify a credential according to the options
    async fn verify_credential(
        &self,
        credential: &VerifiableCredential,
        options: &VerificationOptions,
    ) -> Result<VerificationResult>;
    
    /// Verify a presentation according to the options
    async fn verify_presentation(
        &self,
        presentation: &VerifiablePresentation,
        options: &VerificationOptions,
    ) -> Result<VerificationResult>;
}

/// Basic implementation of a verifier
pub struct BasicVerifier {
    /// Resolver for DIDs
    did_resolver: Box<dyn DidResolver + Send + Sync>,
}

impl BasicVerifier {
    /// Create a new basic verifier
    pub fn new(did_resolver: Box<dyn DidResolver + Send + Sync>) -> Self {
        BasicVerifier {
            did_resolver,
        }
    }
    
    /// Verify the signature on a credential
    async fn verify_signature(
        &self,
        credential: &VerifiableCredential,
    ) -> Result<bool> {
        // Get the proof
        let proof = match &credential.proof {
            Some(proof) => proof,
            None => return Ok(false),
        };
        
        // Get the DID document for the issuer
        let did = Did::parse(&credential.issuer)?;
        let did_document = self.did_resolver.resolve(&did).await?;
        
        // Find the verification method
        let verification_method = did_document
            .get_verification_method(&proof.verification_method)
            .ok_or_else(|| Error::validation("Verification method not found"))?;
        
        // Get the public key from the verification method
        let public_key = verification_method.public_key()?;
        
        // Create a copy of the credential without the proof
        let mut credential_without_proof = credential.clone();
        credential_without_proof.proof = None;
        
        // Serialize the credential for verification
        let data_to_verify = serde_json::to_string(&credential_without_proof)?;
        
        // Verify the signature
        match &proof.value {
            Some(signature) => public_key.verify(data_to_verify.as_bytes(), signature),
            None => Ok(false),
        }
    }
    
    /// Check if a credential is expired
    fn check_expiration(&self, credential: &VerifiableCredential) -> bool {
        match credential.expiration_date {
            Some(expiration) => Utc::now() < expiration,
            None => true, // No expiration date means not expired
        }
    }
}

#[async_trait]
impl Verifier for BasicVerifier {
    async fn verify_credential(
        &self,
        credential: &VerifiableCredential,
        options: &VerificationOptions,
    ) -> Result<VerificationResult> {
        let mut result = VerificationResult {
            verified: true,
            checks: Vec::new(),
            errors: Vec::new(),
        };
        
        // Verify signature if requested
        if options.verify_signature {
            match self.verify_signature(credential).await {
                Ok(true) => {
                    result.add_check(VerificationCheck::SignatureValid);
                }
                Ok(false) => {
                    result.add_error("Invalid signature".to_string());
                }
                Err(e) => {
                    result.add_error(format!("Error verifying signature: {}", e));
                }
            }
        }
        
        // Check expiration if requested
        if options.verify_expiration {
            if self.check_expiration(credential) {
                result.add_check(VerificationCheck::NotExpired);
            } else {
                result.add_error("Credential is expired".to_string());
            }
        }
        
        // Revocation check would go here, but we'll implement it later
        
        // Return the verification result
        Ok(result)
    }
    
    async fn verify_presentation(
        &self,
        presentation: &VerifiablePresentation,
        options: &VerificationOptions,
    ) -> Result<VerificationResult> {
        let mut result = VerificationResult {
            verified: true,
            checks: Vec::new(),
            errors: Vec::new(),
        };
        
        // Verify the presentation signature
        // This would be similar to verifying a credential signature
        
        // Verify that the presentation has the right challenge and domain
        if let Some(expected_challenge) = &options.challenge {
            if let Some(proof) = &presentation.proof {
                if let Some(challenge) = &proof.challenge {
                    if challenge != expected_challenge {
                        result.add_error(format!("Challenge mismatch: expected {}, got {}", 
                                                expected_challenge, challenge));
                    }
                } else {
                    result.add_error("Missing challenge in proof".to_string());
                }
            } else {
                result.add_error("Missing proof in presentation".to_string());
            }
        }
        
        if let Some(expected_domain) = &options.domain {
            if let Some(proof) = &presentation.proof {
                if let Some(domain) = &proof.domain {
                    if domain != expected_domain {
                        result.add_error(format!("Domain mismatch: expected {}, got {}", 
                                                expected_domain, domain));
                    }
                } else {
                    result.add_error("Missing domain in proof".to_string());
                }
            }
        }
        
        // Verify each credential in the presentation
        for credential in &presentation.verifiable_credential {
            match self.verify_credential(credential, options).await {
                Ok(credential_result) => {
                    if !credential_result.verified {
                        result.verified = false;
                        for error in credential_result.errors {
                            result.add_error(format!("Credential verification failed: {}", error));
                        }
                    }
                },
                Err(e) => {
                    result.add_error(format!("Error verifying credential: {}", e));
                }
            }
        }
        
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CredentialSubject;
    use crate::issuer::{BasicIssuer, IssuanceOptions, Issuer};
    use icn_did::DidDocumentResolver;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_verify_credential() {
        // Create an issuer
        let issuer = BasicIssuer::generate("did:icn:test:issuer").unwrap();
        
        // Create a simple DID resolver that only knows about our issuer
        let did_document = issuer.did_document().await.unwrap();
        let resolver = Arc::new(DidDocumentResolver::new(vec![(issuer.did().to_string(), did_document)]));
        
        // Create a verifier with our resolver
        let verifier = BasicVerifier::new(Box::new(resolver));
        
        // Create a credential
        let mut subject = CredentialSubject::new(Some("did:icn:test:subject".to_string()));
        subject.add_property("name", "Alice");
        
        let options = IssuanceOptions::default();
        let credential = issuer.issue_credential(
            None,
            vec!["TestCredential".to_string()],
            subject,
            options,
        ).await.unwrap();
        
        // Verify the credential
        let verification_options = VerificationOptions::default();
        let result = verifier.verify_credential(&credential, &verification_options).await.unwrap();
        
        assert!(result.verified);
        assert!(result.checks.contains(&VerificationCheck::SignatureValid));
        assert!(result.checks.contains(&VerificationCheck::NotExpired));
        assert!(result.errors.is_empty());
    }
} 