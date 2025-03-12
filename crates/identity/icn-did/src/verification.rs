use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use icn_common::error::{Error, Result};
use icn_crypto::{KeyType, PublicKey, Signature, KeyPair};
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

/// A trait for verification methods
#[async_trait]
pub trait Verifier: Send + Sync {
    /// Verify a signature using this verification method
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool>;
}

/// Public key material for verification methods
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PublicKeyMaterial {
    /// Public key in JWK format
    #[serde(rename = "publicKeyJwk")]
    Jwk(serde_json::Value),
    
    /// Public key in base64 format
    #[serde(rename = "publicKeyBase64")]
    Base64(String),
    
    /// Public key in hex format
    #[serde(rename = "publicKeyHex")]
    Hex(String),
    
    /// Public key in PEM format
    #[serde(rename = "publicKeyPem")]
    Pem(String),
    
    /// Public key using multibase encoding
    #[serde(rename = "publicKeyMultibase")]
    Multibase(String),
}

impl PublicKeyMaterial {
    /// Create public key material from a public key
    pub fn from_public_key(public_key: &PublicKey) -> Result<Self> {
        // For now, we'll use base64 format for all key types
        let key_bytes = public_key.to_bytes();
        Ok(PublicKeyMaterial::Base64(base64::encode(key_bytes)))
    }
    
    /// Get the public key from this material
    pub fn to_public_key(&self) -> Result<PublicKey> {
        match self {
            Self::Base64(base64_str) => {
                let key_bytes = base64::decode(base64_str)
                    .map_err(|e| Error::validation(format!("Invalid Base64 public key: {}", e)))?;
                
                match key_bytes.len() {
                    32 => Ok(PublicKey::Ed25519(key_bytes.try_into().unwrap())),
                    _ => Err(Error::validation(format!("Unsupported key length: {}", key_bytes.len()))),
                }
            },
            Self::Hex(hex_str) => {
                let key_bytes = hex::decode(hex_str)
                    .map_err(|e| Error::validation(format!("Invalid Hex public key: {}", e)))?;
                
                match key_bytes.len() {
                    32 => Ok(PublicKey::Ed25519(key_bytes.try_into().unwrap())),
                    _ => Err(Error::validation(format!("Unsupported key length: {}", key_bytes.len()))),
                }
            },
            _ => Err(Error::validation("Unsupported public key format")),
        }
    }
}

/// A public key for verification
pub trait KeyMaterial {
    /// Get the public key
    fn public_key(&self) -> Result<PublicKey>;
    
    /// Get the key type
    fn key_type(&self) -> KeyType;
}

impl KeyMaterial for PublicKeyMaterial {
    fn public_key(&self) -> Result<PublicKey> {
        self.to_public_key()
    }
    
    fn key_type(&self) -> KeyType {
        // For now, we'll assume all keys are Ed25519
        KeyType::Ed25519
    }
}

/// Ed25519 verifier implementation
pub struct Ed25519Verifier {
    /// The public key
    public_key: PublicKey,
}

impl Ed25519Verifier {
    /// Create a new Ed25519 verifier
    pub fn new(public_key: PublicKey) -> Result<Self> {
        if !matches!(public_key, PublicKey::Ed25519(_)) {
            return Err(Error::validation("Not an Ed25519 public key"));
        }
        
        Ok(Self { public_key })
    }
}

#[async_trait]
impl Verifier for Ed25519Verifier {
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        match signature {
            Signature::Ed25519(sig) => {
                match &self.public_key {
                    PublicKey::Ed25519(pk) => {
                        // Use the icn_crypto implementation to verify
                        let result = ed25519_dalek::Verifier::verify(
                            &ed25519_dalek::VerifyingKey::from_bytes(pk)
                                .map_err(|e| Error::validation(format!("Invalid Ed25519 public key: {}", e)))?,
                            message,
                            &ed25519_dalek::Signature::from_bytes(sig)
                                .map_err(|e| Error::validation(format!("Invalid Ed25519 signature: {}", e)))?,
                        );
                        
                        Ok(result.is_ok())
                    },
                    _ => Err(Error::validation("Public key type mismatch")),
                }
            },
            _ => Err(Error::validation("Signature type mismatch")),
        }
    }
}

/// Create a verifier for a verification method
pub fn create_verifier(public_key_material: &PublicKeyMaterial) -> Result<Box<dyn Verifier>> {
    // Get the public key from the material
    let public_key = public_key_material.to_public_key()?;
    
    // Create a verifier based on the key type
    match public_key {
        PublicKey::Ed25519(_) => {
            let verifier = Ed25519Verifier::new(public_key)?;
            Ok(Box::new(verifier))
        },
        _ => Err(Error::validation("Unsupported key type")),
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
    
    #[tokio::test]
    async fn test_ed25519_verification() {
        // Generate a key pair
        let key_pair = KeyPair::generate(KeyType::Ed25519).unwrap();
        
        // Create public key material
        let pk_material = PublicKeyMaterial::from_public_key(&key_pair.public_key()).unwrap();
        
        // Create a verifier
        let verifier = create_verifier(&pk_material).unwrap();
        
        // Test message
        let message = b"Test message";
        
        // Create a signature
        let signature = key_pair.sign(message).unwrap();
        
        // Verify the signature
        let result = verifier.verify(message, &signature).await.unwrap();
        assert!(result);
        
        // Modify the message and verify (should fail)
        let modified_message = b"Modified message";
        let result = verifier.verify(modified_message, &signature).await.unwrap();
        assert!(!result);
    }
    
    #[test]
    fn test_public_key_material_conversion() {
        // Generate a key pair
        let key_pair = KeyPair::generate(KeyType::Ed25519).unwrap();
        let public_key = key_pair.public_key();
        
        // Convert to material
        let material = PublicKeyMaterial::from_public_key(&public_key).unwrap();
        
        // Convert back to public key
        let converted = material.to_public_key().unwrap();
        
        // Should match
        assert_eq!(public_key.to_bytes(), converted.to_bytes());
    }
} 