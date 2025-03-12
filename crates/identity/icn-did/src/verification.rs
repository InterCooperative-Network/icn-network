use async_trait::async_trait;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

/// Verification interface for verification methods
#[async_trait]
pub trait DidVerifierTrait: Send + Sync {
    /// Verify a signature using this verification method
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool>;
}

/// Public key material for verification methods
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum PublicKeyMaterial {
    /// Raw public key bytes
    #[serde(rename = "Raw")]
    Raw(Vec<u8>),
    
    /// Base64-encoded public key
    #[serde(rename = "Base64")]
    Base64(String),
    
    /// Hex-encoded public key
    #[serde(rename = "Hex")]
    Hex(String),
    
    /// Multibase-encoded public key
    #[serde(rename = "Multibase")]
    Multibase {
        /// The multibase-encoded key
        key: String,
    },
    
    /// Ed25519 verification key 2020
    #[serde(rename = "Ed25519VerificationKey2020")]
    Ed25519VerificationKey2020 {
        /// The base58-encoded key
        key: String,
    },
    
    /// Multibase-encoded key with additional properties
    #[serde(rename = "MultibaseKey")]
    MultibaseKey {
        /// The multibase-encoded key
        key: String,
        /// Additional properties
        #[serde(flatten)]
        properties: HashMap<String, serde_json::Value>,
    },
}

impl PublicKeyMaterial {
    /// Create public key material from a public key
    pub fn from_public_key(public_key: &PublicKey) -> Result<Self> {
        match public_key {
            PublicKey::Ed25519(pk) => {
                let key = bs58::encode(pk.as_bytes()).into_string();
                Ok(Self::Ed25519VerificationKey2020 { key })
            },
            _ => Err(Error::validation("Unsupported public key type")),
        }
    }
    
    /// Convert to a public key
    pub fn to_public_key(&self) -> Result<PublicKey> {
        match self {
            Self::Raw(bytes) => {
                parse_key_bytes(bytes.clone(), None)
            },
            Self::Base64(base64_str) => {
                let key_bytes = base64::decode(base64_str)
                    .map_err(|e| Error::validation(format!("Invalid Base64 public key: {}", e)))?;
                
                parse_key_bytes(key_bytes, None)
            },
            Self::Hex(hex_str) => {
                let key_bytes = hex::decode(hex_str)
                    .map_err(|e| Error::validation(format!("Invalid Hex public key: {}", e)))?;
                
                parse_key_bytes(key_bytes, None)
            },
            Self::Multibase { key } => {
                let (_, data) = multibase::decode(key)
                    .map_err(|e| Error::validation(format!("Invalid Multibase public key: {}", e)))?;
                
                parse_key_bytes(data, None)
            },
            Self::Ed25519VerificationKey2020 { key } => {
                let key_bytes = bs58::decode(key)
                    .into_vec()
                    .map_err(|e| Error::validation(format!("Invalid Base58 public key: {}", e)))?;
                
                parse_key_bytes(key_bytes, Some(KeyType::Ed25519))
            },
            Self::MultibaseKey { key, .. } => {
                let (_, data) = multibase::decode(key)
                    .map_err(|e| Error::validation(format!("Invalid Multibase public key: {}", e)))?;
                
                parse_key_bytes(data, None)
            },
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
impl DidVerifierTrait for Ed25519Verifier {
    async fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        match signature {
            Signature::Ed25519(sig) => {
                match &self.public_key {
                    PublicKey::Ed25519(pk) => {
                        // Use the icn_crypto implementation directly
                        let result = self.public_key.verify(message, signature)?;
                        Ok(result)
                    },
                    _ => Err(Error::validation("Public key type mismatch")),
                }
            },
            _ => Err(Error::validation("Signature type mismatch")),
        }
    }
}

/// Create a verifier for a verification method
pub fn create_verifier(public_key_material: &PublicKeyMaterial) -> Result<Box<dyn DidVerifierTrait>> {
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

fn parse_key_bytes(key_bytes: Vec<u8>, key_type: Option<KeyType>) -> Result<PublicKey> {
    // If we know the key type, use it
    if let Some(key_type) = key_type {
        match key_type {
            KeyType::Ed25519 => {
                if key_bytes.len() != 32 {
                    return Err(Error::validation("Invalid Ed25519 key length"));
                }
                let key_array: [u8; 32] = key_bytes.try_into()
                    .map_err(|_| Error::validation("Failed to convert key bytes to array"))?;
                Ok(PublicKey::Ed25519(icn_crypto::ed25519::public_key_from_bytes(&key_array)?))
            },
            _ => Err(Error::validation("Unsupported key type")),
        }
    } else {
        // Try to guess the key type from the length
        match key_bytes.len() {
            32 => {
                let key_array: [u8; 32] = key_bytes.try_into()
                    .map_err(|_| Error::validation("Failed to convert key bytes to array"))?;
                Ok(PublicKey::Ed25519(icn_crypto::ed25519::public_key_from_bytes(&key_array)?))
            },
            _ => Err(Error::validation("Unsupported or unknown key length")),
        }
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