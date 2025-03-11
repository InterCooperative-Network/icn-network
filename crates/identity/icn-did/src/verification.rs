use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use icn_common::error::{Error, Result};
use icn_crypto::Signature;
use icn_crypto::signature::Verifier;

/// Core trait for DID verification
#[async_trait]
pub trait DidVerifier: Send + Sync {
    /// Verify a signature
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool>;
    
    /// Get the method type
    fn method_type(&self) -> &str;
    
    /// Get the public key material
    fn public_key_material(&self) -> &PublicKeyMaterial;
}

/// Generate a random nonce
pub fn generate_nonce() -> String {
    let mut rng = thread_rng();
    let mut nonce = [0u8; 16];
    rng.fill(&mut nonce);
    hex::encode(nonce)
}

/// Authentication challenge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationChallenge {
    /// The DID to authenticate
    pub did: String,
    
    /// The verification method to use
    pub verification_method: String,
    
    /// Random nonce
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

impl std::fmt::Display for PublicKeyMaterial {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ed25519VerificationKey2020 { key } => {
                write!(f, "Ed25519VerificationKey2020 ({})", key)
            }
            Self::JsonWebKey2020 { key } => {
                write!(f, "JsonWebKey2020 ({})", key)
            }
            Self::MultibaseKey { key } => {
                write!(f, "MultibaseKey ({})", key)
            }
        }
    }
}

/// Result of a verification operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    /// Whether the verification was successful
    pub verified: bool,
    
    /// The verification method used
    pub verification_method: String,
    
    /// The public key material used
    pub public_key: PublicKeyMaterial,
}

/// Implementation specific verifiers
pub struct Ed25519Verifier {
    public_key: Vec<u8>, // Store the raw bytes instead of the PublicKey type
    key_material: PublicKeyMaterial,
}

impl Ed25519Verifier {
    pub fn new(public_key: Vec<u8>, key_material: PublicKeyMaterial) -> Self {
        Self {
            public_key,
            key_material,
        }
    }
}

#[async_trait]
impl DidVerifier for Ed25519Verifier {
    async fn verify(&self, _message: &[u8], _signature: &Signature) -> Result<bool> {
        // For now, just return true as we need to implement proper verification
        Ok(true)
    }
    
    fn method_type(&self) -> &str {
        "Ed25519VerificationKey2020"
    }
    
    fn public_key_material(&self) -> &PublicKeyMaterial {
        &self.key_material
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_challenge_creation() {
        let challenge = AuthenticationChallenge::new(
            "did:icn:test",
            "did:icn:test#key-1",
            3600
        ).unwrap();
        
        assert_eq!(challenge.did, "did:icn:test");
        assert_eq!(challenge.verification_method, "did:icn:test#key-1");
        assert!(!challenge.nonce.is_empty());
        assert!(challenge.expires > challenge.issued);
        assert_eq!(challenge.expires - challenge.issued, 3600);
    }
    
    #[test]
    fn test_challenge_expiration() {
        let mut challenge = AuthenticationChallenge::new(
            "did:icn:test",
            "did:icn:test#key-1",
            3600
        ).unwrap();
        
        assert!(!challenge.is_expired().unwrap());
        
        // Set expiration to the past
        challenge.expires = 0;
        assert!(challenge.is_expired().unwrap());
    }
    
    #[test]
    fn test_challenge_message() {
        let challenge = AuthenticationChallenge::new(
            "did:icn:test",
            "did:icn:test#key-1",
            3600
        ).unwrap();
        
        let message = challenge.get_message();
        assert!(!message.is_empty());
        
        let expected = format!(
            "{}:{}:{}:{}:{}",
            challenge.did,
            challenge.verification_method,
            challenge.nonce,
            challenge.issued,
            challenge.expires
        ).into_bytes();
        
        assert_eq!(message, expected);
    }
} 