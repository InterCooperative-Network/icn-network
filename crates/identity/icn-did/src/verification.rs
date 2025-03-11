//! Verification types and utilities for DID documents
//! 
//! This module provides types and utilities for verifying DID documents
//! and signatures made with DID verification methods.

use async_trait::async_trait;
use icn_common::{Error, Result};
use icn_crypto::PublicKey;
use icn_crypto::Signature;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{thread_rng, Rng};

/// Core trait for DID verification
#[async_trait]
pub trait DidVerifier: Send + Sync {
    /// Verify a signature using this verifier
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool>;
    
    /// Get the verification method type
    fn method_type(&self) -> &str;
    
    /// Get the public key material
    fn public_key_material(&self) -> &PublicKeyMaterial;
}

/// Result of verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub method_id: String,
    pub timestamp: u64,
}

/// A challenge used for DID authentication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationChallenge {
    /// The DID being authenticated
    pub did: String,
    
    /// The verification method to use
    pub verification_method: String,
    
    /// A nonce to prevent replay attacks
    pub nonce: String,
    
    /// When the challenge was issued (Unix timestamp)
    pub issued: u64,
    
    /// When the challenge expires (Unix timestamp)
    pub expires: u64,
}

impl AuthenticationChallenge {
    /// Create a new authentication challenge
    pub fn new(did: &str, verification_method: &str, ttl_secs: u64) -> Result<Self> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("Failed to get system time: {}", e)))?
            .as_secs();
            
        Ok(Self {
            did: did.to_string(),
            verification_method: verification_method.to_string(),
            nonce: generate_nonce(),
            issued: now,
            expires: now + ttl_secs,
        })
    }
    
    /// Check if the challenge is expired
    pub fn is_expired(&self) -> Result<bool> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .map_err(|e| Error::internal(format!("Failed to get system time: {}", e)))?
            .as_secs();
            
        Ok(self.expires < now)
    }
    
    /// Get the message to sign
    pub fn get_message(&self) -> Vec<u8> {
        self.to_signing_input().unwrap_or_default()
    }
    
    /// Convert the challenge to a signing input
    fn to_signing_input(&self) -> Result<Vec<u8>> {
        let input = format!(
            "{}:{}:{}:{}:{}",
            self.did,
            self.verification_method,
            self.nonce,
            self.issued,
            self.expires
        );
        
        Ok(input.into_bytes())
    }
}

/// A signed authentication response
#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticationResponse {
    /// The original challenge
    pub challenge: AuthenticationChallenge,
    
    /// The signature over the challenge
    pub signature: Vec<u8>,
}

/// Types of public key material supported
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

/// A verification method in a DID document
#[derive(Debug, Clone)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub type_: String,
    pub public_key: PublicKeyMaterial,
}

impl VerificationMethod {
    pub fn new(
        id: String,
        controller: String,
        type_: String,
        public_key: PublicKeyMaterial,
    ) -> Self {
        Self {
            id,
            controller,
            type_,
            public_key,
        }
    }

    pub fn verify_signature(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        match &self.public_key {
            PublicKeyMaterial::Ed25519VerificationKey2020(key) => {
                let key_bytes = bs58::decode(key)
                    .into_vec()
                    .map_err(|e| Error::validation(format!("Invalid base58 key: {}", e)))?;
                
                let public_key = icn_crypto::ed25519::PublicKey::from_bytes(&key_bytes)?;
                public_key.verify(message, signature)
            }
            PublicKeyMaterial::JsonWebKey2020(_) => {
                Err(Error::not_implemented("JWK verification not implemented"))
            }
            PublicKeyMaterial::MultibaseKey(key) => {
                let key_bytes = multibase::decode(key)
                    .map_err(|e| Error::validation(format!("Invalid multibase key: {}", e)))?
                    .1;
                
                let public_key = icn_crypto::ed25519::PublicKey::from_bytes(&key_bytes)?;
                public_key.verify(message, signature)
            }
        }
    }
}

/// Implementation specific verifiers
pub struct Ed25519Verifier {
    public_key: icn_crypto::ed25519::PublicKey,
    key_material: PublicKeyMaterial,
}

impl Ed25519Verifier {
    pub fn new(public_key: icn_crypto::ed25519::PublicKey, key_material: PublicKeyMaterial) -> Self {
        Self {
            public_key,
            key_material,
        }
    }
}

#[async_trait]
impl DidVerifier for Ed25519Verifier {
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        self.public_key.verify(message, signature)
    }
    
    fn method_type(&self) -> &str {
        "Ed25519VerificationKey2020"
    }
    
    fn public_key_material(&self) -> &PublicKeyMaterial {
        &self.key_material
    }
}

/// Generate a random nonce
fn generate_nonce() -> String {
    let mut rng = thread_rng();
    let nonce: [u8; 16] = rng.gen();
    hex::encode(nonce)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_challenge_creation() {
        let challenge = AuthenticationChallenge::new("did:icn:123", "did:icn:123#key1", 3600).unwrap();
        
        assert_eq!(challenge.did, "did:icn:123");
        assert_eq!(challenge.verification_method, "did:icn:123#key1");
        assert!(!challenge.nonce.is_empty());
        assert!(challenge.expires > challenge.issued);
        assert_eq!(challenge.expires - challenge.issued, 3600);
    }
    
    #[test]
    fn test_challenge_expiration() {
        // Create a challenge that's already expired
        let mut challenge = AuthenticationChallenge::new("did:icn:123", "did:icn:123#key1", 3600).unwrap();
        challenge.expires = challenge.issued - 1;
        
        assert!(challenge.is_expired().unwrap());
        
        // Create a challenge that's not expired
        let challenge = AuthenticationChallenge::new("did:icn:123", "did:icn:123#key1", 3600).unwrap();
        assert!(!challenge.is_expired().unwrap());
    }
}