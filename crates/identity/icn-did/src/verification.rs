//! Verification types and utilities for DID documents
//! 
//! This module provides types and utilities for verifying DID documents
//! and signatures made with DID verification methods.

use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use icn_common::{Error, Result};
use icn_crypto::{KeyType, PublicKey, Signature, Verifier};
use icn_crypto::{ed25519, signature::Signature};
use icn_crypto::{hash::Hash, signature::Signature};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::system_time("Failed to get system time"))?
            .as_secs();
            
        Ok(Self {
            did: did.to_string(),
            verification_method: verification_method.to_string(),
            nonce: generate_nonce(),
            issued: now,
            expires: now + ttl_secs,
        })
    }
    
    /// Check if the challenge has expired
    pub fn is_expired(&self) -> Result<bool> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| Error::system_time("Failed to get system time"))?
            .as_secs();
            
        Ok(now > self.expires)
    }
    
    /// Get the challenge message to sign
    pub fn get_message(&self) -> Vec<u8> {
        // Challenge message format:
        // did:verification_method:nonce:issued:expires
        format!("{}:{}:{}:{}:{}",
            self.did,
            self.verification_method,
            self.nonce,
            self.issued,
            self.expires
        ).into_bytes()
    }

    pub fn verify_signature(&self, public_key: &[u8], signature: &[u8]) -> Result<bool> {
        // Create message that was signed
        let message = self.to_signing_input()?;
        
        // Verify Ed25519 signature
        let sig = Signature::from_bytes(signature)
            .map_err(|e| Error::validation(format!("Invalid signature: {}", e)))?;
            
        let pk = ed25519::PublicKey::from_bytes(public_key)
            .map_err(|e| Error::validation(format!("Invalid public key: {}", e)))?;

        Ok(pk.verify(&message, &sig))
    }

    fn to_signing_input(&self) -> Result<Vec<u8>> {
        // Combine nonce and timestamp into message
        let message = format!("{}:{}", self.nonce, self.timestamp);
        Ok(message.into_bytes())
    }
}

/// A signed authentication response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationResponse {
    /// The original challenge
    pub challenge: AuthenticationChallenge,
    
    /// The signature over the challenge message
    pub signature: Signature,
}

/// Generate a random nonce
fn generate_nonce() -> String {
    use rand::{thread_rng, Rng};
    let mut rng = thread_rng();
    let bytes: [u8; 32] = rng.gen();
    hex::encode(bytes)
}

#[derive(Debug, Clone)]
pub struct VerificationMethod {
    pub id: String,
    pub controller: String,
    pub type_: String,
    pub public_key: PublicKeyMaterial,
}

#[derive(Debug, Clone)]
pub enum PublicKeyMaterial {
    Ed25519VerificationKey2020(String), // base58 encoded
    JsonWebKey2020(String),             // JWK format
    MultibaseKey(String),               // Multibase encoded
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
        let public_key = match &self.public_key {
            PublicKeyMaterial::Ed25519VerificationKey2020(key) => {
                // Decode base58 key
                let key_bytes = bs58::decode(key)
                    .into_vec()
                    .map_err(|e| Error::validation(format!("Invalid base58 key: {}", e)))?;
                
                icn_crypto::ed25519::Ed25519PublicKey::from_bytes(&key_bytes)?
            }
            PublicKeyMaterial::JsonWebKey2020(_) => {
                return Err(Error::not_implemented("JWK verification not implemented"))
            }
            PublicKeyMaterial::MultibaseKey(_) => {
                return Err(Error::not_implemented("Multibase verification not implemented"))
            }
        };

        public_key.verify(message, signature)
    }
}

#[derive(Debug, Clone)]
pub struct Challenge {
    pub nonce: String,
    pub method_id: String,
    pub timestamp: u64,
}

impl Challenge {
    pub fn new(method_id: String) -> Self {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        let nonce: [u8; 32] = rng.gen();
        
        Self {
            nonce: bs58::encode(nonce).into_string(),
            method_id,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    pub fn verify_response(&self, method: &VerificationMethod, signature: &Signature) -> Result<bool> {
        let message = self.to_bytes();
        method.verify_signature(&message, signature)
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(self.nonce.as_bytes());
        bytes.extend_from_slice(self.method_id.as_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use icn_crypto::ed25519::KeyPair;
    
    #[test]
    fn test_challenge_creation() {
        let did = "did:icn:123";
        let verification_method = "#keys-1";
        let ttl = 300; // 5 minutes
        
        let challenge = AuthenticationChallenge::new(did, verification_method, ttl).unwrap();
        
        assert_eq!(challenge.did, did);
        assert_eq!(challenge.verification_method, verification_method);
        assert!(!challenge.nonce.is_empty());
        assert!(challenge.expires > challenge.issued);
        assert_eq!(challenge.expires - challenge.issued, ttl);
    }
    
    #[test]
    fn test_challenge_expiration() {
        let challenge = AuthenticationChallenge::new(
            "did:icn:123",
            "#keys-1",
            0 // Expire immediately
        ).unwrap();
        
        assert!(challenge.is_expired().unwrap());
        
        let challenge = AuthenticationChallenge::new(
            "did:icn:123",
            "#keys-1",
            3600 // 1 hour
        ).unwrap();
        
        assert!(!challenge.is_expired().unwrap());
    }
    
    #[test]
    fn test_challenge_message() {
        let challenge = AuthenticationChallenge {
            did: "did:icn:123".to_string(),
            verification_method: "#keys-1".to_string(),
            nonce: "abc123".to_string(),
            issued: 1000,
            expires: 2000,
        };
        
        let message = challenge.get_message();
        let expected = b"did:icn:123:#keys-1:abc123:1000:2000";
        assert_eq!(message, expected);
    }
    
    #[test]
    fn test_nonce_generation() {
        let nonce1 = generate_nonce();
        let nonce2 = generate_nonce();
        
        assert!(!nonce1.is_empty());
        assert_eq!(nonce1.len(), 64); // 32 bytes hex encoded
        assert_ne!(nonce1, nonce2); // Should be random
    }

    #[test]
    fn test_authentication_challenge() {
        // Generate a key pair for testing
        let keypair = KeyPair::generate();
        
        // Create challenge
        let challenge = AuthenticationChallenge::new("test-method".to_string());
        
        // Sign challenge
        let message = challenge.to_signing_input().unwrap();
        let signature = keypair.sign(&message);
        
        // Verify signature
        assert!(challenge.verify_signature(
            keypair.public_key().as_bytes(),
            signature.as_bytes()
        ).unwrap());
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = KeyPair::generate();
        let challenge = AuthenticationChallenge::new("test-method".to_string());
        
        // Create invalid signature
        let invalid_sig = vec![0u8; 64];
        
        // Verification should fail
        assert!(!challenge.verify_signature(
            keypair.public_key().as_bytes(),
            &invalid_sig
        ).unwrap());
    }
}