//! Decentralized Identifier (DID) implementation for ICN
//!
//! This crate implements the W3C DID specification for the ICN project,
//! providing identity management capabilities.

pub mod resolver;
pub mod manager;
pub mod verification;

use icn_common::{Error, Result};
use icn_crypto::{PublicKey, Signature, Verifier, KeyType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The ICN DID method name
pub const DID_METHOD: &str = "icn";

// Re-export commonly used types
pub use resolver::{DidResolver, ResolutionResult};
pub use manager::{DidManager, DidManagerConfig, CreateDidOptions};
pub use verification::*;

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

    /// Get a verification method by ID
    pub fn get_verification_method(&self, id: &str) -> Option<&VerificationMethod> {
        let full_id = if id.starts_with(&self.id) {
            id.to_string()
        } else {
            format!("{}#{}", self.id, id.trim_start_matches('#'))
        };

        self.verification_method.iter()
            .find(|m| m.id == full_id)
    }

    /// Get a verification method by ID and verify it's usable for authentication
    pub fn get_authentication_method(&self, id: &str) -> Option<&VerificationMethod> {
        let method = self.get_verification_method(id)?;
        
        // Check if method is listed in authentication
        if !self.authentication.iter().any(|r| match r {
            VerificationMethodReference::Reference(ref_id) => ref_id == &method.id,
            VerificationMethodReference::Embedded(vm) => vm.id == method.id,
        }) {
            return None;
        }

        Some(method)
    }

    /// Verify a signature using a specific verification method
    pub fn verify_signature(
        &self,
        method_id: &str,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        let method = self.get_verification_method(method_id)
            .ok_or_else(|| Error::not_found("Verification method not found"))?;

        let verifier = self.create_verifier(method)?;
        verifier.verify(message, signature)
    }

    /// Verify a signature for authentication purposes
    pub fn verify_authentication(
        &self,
        method_id: &str,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        let method = self.get_authentication_method(method_id)
            .ok_or_else(|| Error::not_found("Authentication method not found"))?;

        let verifier = self.create_verifier(method)?;
        verifier.verify(message, signature)
    }

    /// Create a verifier for a verification method
    fn create_verifier(&self, method: &VerificationMethod) -> Result<Box<dyn Verifier>> {
        match &method.public_key {
            PublicKeyMaterial::Ed25519VerificationKey2020(key) => {
                let public_key = icn_crypto::decode_public_key(KeyType::Ed25519, key)?;
                Ok(Box::new(icn_crypto::Ed25519Verifier::new(public_key)))
            }
            PublicKeyMaterial::JsonWebKey2020(_) => {
                Err(Error::not_implemented("JWK verification not implemented"))
            }
            PublicKeyMaterial::MultibaseKey(key) => {
                let public_key = icn_crypto::decode_multibase_key(key)?;
                Ok(Box::new(icn_crypto::Ed25519Verifier::new(public_key)))
            }
        }
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

    #[test]
    fn test_verification_method_lookup() {
        let mut doc = DidDocument::new("123456789").unwrap();
        let method = VerificationMethod {
            id: format!("{}#keys-1", doc.id),
            type_: "Ed25519VerificationKey2020".to_string(),
            controller: doc.id.clone(),
            public_key: PublicKeyMaterial::Ed25519VerificationKey2020(
                "BASE58_PUBLIC_KEY".to_string()
            ),
        };

        doc.add_verification_method(method.clone());
        doc.add_authentication(VerificationMethodReference::Embedded(method.clone()));

        // Test full ID lookup
        let found = doc.get_verification_method(&method.id);
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, method.id);

        // Test fragment lookup
        let found = doc.get_verification_method("#keys-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, method.id);

        // Test authentication method lookup
        let found = doc.get_authentication_method("#keys-1");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, method.id);
    }
}
