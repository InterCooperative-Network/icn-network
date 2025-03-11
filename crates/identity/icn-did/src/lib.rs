//! Decentralized Identifier (DID) implementation for ICN
//!
//! This crate implements the W3C DID specification for the ICN project,
//! providing identity management capabilities.

use icn_common::{Error, Result};
use icn_crypto::{PublicKey, Signature, Verifier};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The ICN DID method name
pub const DID_METHOD: &str = "icn";

/// A DID document representing an identity in the ICN system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DidDocument {
    /// The DID for this document
    pub id: String,
    
    /// Controller DIDs that can modify this document
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controller: Vec<String>,
    
    /// Verification methods (keys)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub verification_method: Vec<VerificationMethod>,
    
    /// Authentication verification methods
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authentication: Vec<VerificationMethodReference>,
    
    /// Assertion verification methods
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertion_method: Vec<VerificationMethodReference>,
    
    /// Key agreement verification methods
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub key_agreement: Vec<VerificationMethodReference>,
    
    /// Service endpoints
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub service: Vec<Service>,
}

/// A verification method in a DID document
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VerificationMethod {
    /// The ID of this verification method
    pub id: String,
    
    /// The type of verification method
    pub type_: String,
    
    /// The controller of this verification method
    pub controller: String,
    
    /// The public key material
    #[serde(flatten)]
    pub public_key: PublicKeyMaterial,
}

/// Types of public key material
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublicKeyMaterial {
    /// Ed25519 verification key
    #[serde(rename = "publicKeyBase58")]
    Ed25519VerificationKey2020(String),
    
    /// JSON Web Key
    #[serde(rename = "publicKeyJwk")]
    JsonWebKey2020(HashMap<String, serde_json::Value>),
    
    /// Multibase public key
    #[serde(rename = "publicKeyMultibase")]
    MultibaseKey(String),
}

/// A reference to a verification method
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VerificationMethodReference {
    /// Reference by ID
    Reference(String),
    /// Embedded verification method
    Embedded(VerificationMethod),
}

/// A service endpoint in a DID document
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Service {
    /// The ID of this service
    pub id: String,
    
    /// The type of service
    pub type_: String,
    
    /// The service endpoint URL
    pub service_endpoint: String,
}

impl DidDocument {
    /// Create a new DID document with a generated ID
    pub fn new(subject_id: &str) -> Result<Self> {
        Ok(Self {
            id: format!("did:{}:{}", DID_METHOD, subject_id),
            controller: vec![],
            verification_method: vec![],
            authentication: vec![],
            assertion_method: vec![],
            key_agreement: vec![],
            service: vec![],
        })
    }
    
    /// Add a verification method to the DID document
    pub fn add_verification_method(&mut self, method: VerificationMethod) {
        self.verification_method.push(method);
    }
    
    /// Add an authentication reference
    pub fn add_authentication(&mut self, reference: VerificationMethodReference) {
        self.authentication.push(reference);
    }
    
    /// Add a service endpoint
    pub fn add_service(&mut self, service: Service) {
        self.service.push(service);
    }
    
    /// Validate the DID document structure
    pub fn validate(&self) -> Result<()> {
        if !self.id.starts_with(&format!("did:{}:", DID_METHOD)) {
            return Err(Error::validation("Invalid DID format"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_did_document_creation() {
        let did_doc = DidDocument::new("123456789").unwrap();
        assert_eq!(did_doc.id, "did:icn:123456789");
    }
    
    #[test]
    fn test_did_validation() {
        let mut did_doc = DidDocument::new("123456789").unwrap();
        assert!(did_doc.validate().is_ok());
        
        // Test invalid DID
        did_doc.id = "invalid:did".to_string();
        assert!(did_doc.validate().is_err());
    }
}
