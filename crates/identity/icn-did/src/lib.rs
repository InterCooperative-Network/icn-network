//! Decentralized Identifier (DID) implementation for ICN
//!
//! This crate implements the W3C DID specification for the ICN project,
//! providing identity management capabilities.

pub mod resolver;
pub mod manager;
pub mod verification;
pub mod federation;

use icn_common::{Error, Result};
use icn_crypto::{PublicKey, Signature, KeyType};
use icn_crypto::signature::Verifier;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// The ICN DID method name
pub const DID_METHOD: &str = "icn";

// Re-export commonly used types
pub use resolver::{DidResolver, ResolutionResult, DocumentMetadata, ResolutionMetadata};
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

impl DidDocument {
    /// Create a new DID document
    pub fn new(subject_id: &str) -> Result<Self> {
        // Check for empty subject_id
        if subject_id.is_empty() {
            return Err(Error::validation("DID cannot be empty"));
        }
        
        // Handle both full DIDs and identifier-only format
        let id = if subject_id.starts_with(&format!("did:{}:", DID_METHOD)) {
            subject_id.to_string()
        } else {
            format!("did:{}:{}", DID_METHOD, subject_id)
        };
        
        Ok(Self {
            id,
            controller: Vec::new(),
            verification_method: Vec::new(),
            authentication: Vec::new(),
            assertion_method: Vec::new(),
            key_agreement: Vec::new(),
            service: Vec::new(),
        })
    }
    
    /// Add a context to the DID document
    pub fn add_context(&mut self, _context: &str) -> &mut Self {
        // This is a placeholder implementation
        // In a real implementation, we would add the context to a contexts field
        self
    }
    
    /// Add a verification method to the DID document
    pub fn add_verification_method(&mut self, method: VerificationMethod) {
        self.verification_method.push(method);
    }
    
    /// Add an authentication method to the DID document
    pub fn add_authentication(&mut self, reference: VerificationMethodReference) {
        self.authentication.push(reference);
    }
    
    /// Add an assertion method to the DID document
    pub fn add_assertion_method(&mut self, reference: VerificationMethodReference) {
        self.assertion_method.push(reference);
    }
    
    /// Add a key agreement method to the DID document
    pub fn add_key_agreement(&mut self, reference: VerificationMethodReference) {
        self.key_agreement.push(reference);
    }
    
    /// Add a service to the DID document
    pub fn add_service(&mut self, service: Service) {
        self.service.push(service);
    }
    
    /// Get a verification method by ID
    pub fn get_verification_method(&self, id: &str) -> Option<&VerificationMethod> {
        self.verification_method.iter().find(|m| m.id == id)
    }
    
    /// Get an authentication method by ID
    pub fn get_authentication_method(&self, id: &str) -> Option<&VerificationMethod> {
        for auth in &self.authentication {
            match auth {
                VerificationMethodReference::Reference(ref_id) => {
                    if ref_id == id {
                        return self.get_verification_method(ref_id);
                    }
                }
                VerificationMethodReference::Embedded(method) => {
                    if method.id == id {
                        return Some(method);
                    }
                }
            }
        }
        None
    }
    
    /// Verify a signature using a specific verification method
    pub fn verify_signature(
        &self,
        method_id: &str,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        let method = self.get_verification_method(method_id)
            .ok_or_else(|| Error::not_found(format!("Verification method {} not found", method_id)))?;
        
        // For now, we'll just return false as we need to implement proper verification
        // In a real implementation, we would use the method's public key to verify the signature
        Ok(false)
    }
    
    /// Verify a signature for authentication
    pub fn verify_authentication(
        &self,
        method_id: &str,
        message: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        let method = self.get_authentication_method(method_id)
            .ok_or_else(|| Error::not_found(format!("Authentication method {} not found", method_id)))?;
        
        // For now, we'll just return false as we need to implement proper verification
        // In a real implementation, we would use the method's public key to verify the signature
        Ok(false)
    }
    
    /// Create a verifier for a verification method
    fn create_verifier(&self, method: &VerificationMethod) -> Result<Box<dyn Verifier>> {
        // For now, we'll just return an error as we need to implement proper verification
        // In a real implementation, we would create a verifier based on the method's type
        Err(Error::internal("Verifier creation not implemented"))
    }
    
    /// Validate the DID document
    pub fn validate(&self) -> Result<()> {
        // Basic validation
        if self.id.is_empty() {
            return Err(Error::validation("DID document must have an id"));
        }
        
        // Validate verification methods
        for method in &self.verification_method {
            if method.id.is_empty() {
                return Err(Error::validation("Verification method must have an id"));
            }
            if method.type_.is_empty() {
                return Err(Error::validation("Verification method must have a type"));
            }
            if method.controller.is_empty() {
                return Err(Error::validation("Verification method must have a controller"));
            }
        }
        
        // Validate service endpoints
        for service in &self.service {
            if service.id.is_empty() {
                return Err(Error::validation("Service must have an id"));
            }
            
            // Check if the service ID is properly formatted (should start with the document ID or be a fragment)
            if !service.id.starts_with(&self.id) && !service.id.starts_with('#') {
                return Err(Error::validation(format!("Invalid service ID format: {}", service.id)));
            }
            
            if service.type_.is_empty() {
                return Err(Error::validation("Service must have a type"));
            }
            if service.service_endpoint.is_empty() {
                return Err(Error::validation("Service must have an endpoint"));
            }
        }
        
        Ok(())
    }
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PublicKeyMaterial {
    /// Ed25519 verification key
    #[serde(rename = "Ed25519VerificationKey2020")]
    Ed25519VerificationKey2020 {
        #[serde(rename = "publicKeyBase58")]
        key: String,
    },
    
    /// JSON Web Key
    #[serde(rename = "JsonWebKey2020")]
    JsonWebKey2020 {
        #[serde(rename = "publicKeyJwk")]
        key: serde_json::Value,
    },
    
    /// Multibase encoded key
    #[serde(rename = "MultibaseKey")]
    MultibaseKey {
        #[serde(rename = "publicKeyMultibase")]
        key: String,
    },
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

// Implement std::fmt::Display for PublicKeyMaterial
impl fmt::Display for PublicKeyMaterial {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublicKeyMaterial::Ed25519VerificationKey2020 { key } => write!(f, "{}", key),
            PublicKeyMaterial::JsonWebKey2020 { key } => write!(f, "{}", serde_json::to_string(key).unwrap_or_default()),
            PublicKeyMaterial::MultibaseKey { key } => write!(f, "{}", key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_did_document_creation() {
        let did_doc = DidDocument::new("did:icn:123456789").unwrap();
        assert_eq!(did_doc.id, "did:icn:123456789");
        assert!(did_doc.validate().is_ok());
        
        // Test empty subject ID
        assert!(DidDocument::new("").is_err());
    }

    #[test]
    fn test_verification_methods() {
        let mut doc = DidDocument::new("did:icn:123456789").unwrap();
        let method = VerificationMethod {
            id: format!("{}#keys-1", doc.id),
            controller: doc.id.clone(),
            type_: "Ed25519VerificationKey2020".to_string(),
            public_key: PublicKeyMaterial::Ed25519VerificationKey2020 {
                key: "BASE58_PUBLIC_KEY".to_string()
            },
        };
        
        doc.add_verification_method(method.clone());
        assert_eq!(doc.verification_method.len(), 1);
        
        let retrieved = doc.get_verification_method(&format!("{}#keys-1", doc.id)).unwrap();
        assert_eq!(retrieved.id, format!("{}#keys-1", doc.id));
    }

    #[test]
    fn test_service_endpoints() {
        let mut doc = DidDocument::new("did:icn:123456789").unwrap();
        let service = Service {
            id: format!("{}#service-1", doc.id),
            type_: "MessagingService".to_string(),
            service_endpoint: "https://example.com/messaging".to_string(),
        };

        doc.add_service(service);
        assert!(doc.validate().is_ok());
        assert_eq!(doc.service.len(), 1);

        // Test invalid service ID
        let mut doc = DidDocument::new("did:icn:123456789").unwrap();
        let service = Service {
            id: "invalid-id".to_string(),
            type_: "MessagingService".to_string(),
            service_endpoint: "https://example.com/messaging".to_string(),
        };

        doc.add_service(service);
        assert!(doc.validate().is_err());
    }
}
