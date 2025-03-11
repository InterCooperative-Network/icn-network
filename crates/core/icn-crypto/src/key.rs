//! Key types for the ICN crypto system

use std::fmt;
use rand::rngs::OsRng;

use k256::SecretKey as K256SecretKey;
use k256::PublicKey as K256PublicKey;
use k256::elliptic_curve::sec1::ToEncodedPoint;
use ed25519_dalek::{Keypair as Ed25519Keypair, PublicKey as Ed25519PublicKey, SecretKey as Ed25519SecretKey, Signer as Ed25519Signer};

use crate::error::{CryptoError, Result};
use crate::signature::Signature;

/// Type of cryptographic key
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyType {
    /// Ed25519 keys for signing and verification
    Ed25519,
    
    /// Secp256k1 keys for signing and verification
    Secp256k1,
    
    /// X25519 keys for key agreement
    X25519,
}

/// Private key for cryptographic operations
pub enum PrivateKey {
    /// Ed25519 private key
    Ed25519(Ed25519SecretKey),
    
    /// Secp256k1 private key
    Secp256k1(K256SecretKey),
}

impl PrivateKey {
    /// Create a private key from bytes
    pub fn from_bytes(key_type: KeyType, bytes: &[u8]) -> Result<Self> {
        match key_type {
            KeyType::Ed25519 => {
                let secret = Ed25519SecretKey::from_bytes(bytes)
                    .map_err(|e| CryptoError::InvalidKey(format!("Invalid Ed25519 key: {}", e)))?;
                Ok(PrivateKey::Ed25519(secret))
            }
            KeyType::Secp256k1 => {
                Err(CryptoError::UnsupportedAlgorithm("Secp256k1 private key creation not implemented".to_string()))
            }
            KeyType::X25519 => {
                Err(CryptoError::UnsupportedAlgorithm("X25519 private key creation not implemented".to_string()))
            }
        }
    }
    
    /// Get the type of this key
    pub fn key_type(&self) -> KeyType {
        match self {
            PrivateKey::Ed25519(_) => KeyType::Ed25519,
            PrivateKey::Secp256k1(_) => KeyType::Secp256k1,
        }
    }
    
    /// Export this key as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PrivateKey::Ed25519(key) => key.as_bytes().to_vec(),
            PrivateKey::Secp256k1(key) => key.to_be_bytes().as_slice().to_vec(),
        }
    }
}

impl fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrivateKey::Ed25519(_) => write!(f, "Ed25519PrivateKey(REDACTED)"),
            PrivateKey::Secp256k1(_) => write!(f, "Secp256k1PrivateKey(REDACTED)"),
        }
    }
}

/// Public key for cryptographic operations
pub enum PublicKey {
    /// Ed25519 public key
    Ed25519(Ed25519PublicKey),
    
    /// Secp256k1 public key
    Secp256k1(K256PublicKey),
}

impl PublicKey {
    /// Create a public key from bytes
    pub fn from_bytes(key_type: KeyType, bytes: &[u8]) -> Result<Self> {
        match key_type {
            KeyType::Ed25519 => {
                let public = Ed25519PublicKey::from_bytes(bytes)
                    .map_err(|e| CryptoError::InvalidKey(format!("Invalid Ed25519 key: {}", e)))?;
                Ok(PublicKey::Ed25519(public))
            }
            KeyType::Secp256k1 => {
                Err(CryptoError::UnsupportedAlgorithm("Secp256k1 public key creation not implemented".to_string()))
            }
            KeyType::X25519 => {
                Err(CryptoError::UnsupportedAlgorithm("X25519 public key creation not implemented".to_string()))
            }
        }
    }
    
    /// Get the type of this key
    pub fn key_type(&self) -> KeyType {
        match self {
            PublicKey::Ed25519(_) => KeyType::Ed25519,
            PublicKey::Secp256k1(_) => KeyType::Secp256k1,
        }
    }
    
    /// Export this key as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            PublicKey::Ed25519(key) => key.as_bytes().to_vec(),
            PublicKey::Secp256k1(key) => key.to_encoded_point(true).as_bytes().to_vec(),
        }
    }
    
    /// Verify a signature against a message
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool> {
        match (self, signature) {
            (PublicKey::Ed25519(public_key), Signature::Ed25519(sig)) => {
                Ok(public_key.verify_strict(message, sig).is_ok())
            }
            (PublicKey::Secp256k1(_), Signature::Secp256k1(_)) => {
                Err(CryptoError::UnsupportedAlgorithm("Secp256k1 verification not implemented".to_string()))
            }
            _ => Err(CryptoError::InvalidKey("Key type does not match signature type".to_string())),
        }
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PublicKey::Ed25519(key) => write!(f, "Ed25519PublicKey({})", bs58::encode(key.as_bytes()).into_string()),
            PublicKey::Secp256k1(key) => write!(f, "Secp256k1PublicKey({})", hex::encode(key.to_encoded_point(true).as_bytes())),
        }
    }
}

/// Key pair for cryptographic operations
pub enum KeyPair {
    /// Ed25519 key pair
    Ed25519(Ed25519Keypair),
    
    /// Secp256k1 key pair
    Secp256k1 {
        secret_key: K256SecretKey,
        public_key: K256PublicKey,
    },
}

impl KeyPair {
    /// Generate a new key pair
    pub fn generate(key_type: KeyType) -> Result<Self> {
        match key_type {
            KeyType::Ed25519 => {
                let mut csprng = OsRng;
                let keypair = Ed25519Keypair::generate(&mut csprng);
                Ok(KeyPair::Ed25519(keypair))
            }
            KeyType::Secp256k1 => {
                Err(CryptoError::UnsupportedAlgorithm("Secp256k1 key generation not implemented".to_string()))
            }
            KeyType::X25519 => {
                Err(CryptoError::UnsupportedAlgorithm("X25519 key generation not implemented".to_string()))
            }
        }
    }
    
    /// Get the type of this key pair
    pub fn key_type(&self) -> KeyType {
        match self {
            KeyPair::Ed25519(_) => KeyType::Ed25519,
            KeyPair::Secp256k1 { .. } => KeyType::Secp256k1,
        }
    }
    
    /// Get the public key from this key pair
    pub fn public_key(&self) -> PublicKey {
        match self {
            KeyPair::Ed25519(keypair) => PublicKey::Ed25519(keypair.public),
            KeyPair::Secp256k1 { public_key, .. } => PublicKey::Secp256k1(public_key.clone()),
        }
    }
    
    /// Get the private key from this key pair
    pub fn private_key(&self) -> PrivateKey {
        match self {
            KeyPair::Ed25519(keypair) => {
                // We need to create a new secret key since ed25519_dalek doesn't implement Clone
                let secret_bytes = keypair.secret.as_bytes();
                let secret = Ed25519SecretKey::from_bytes(secret_bytes)
                    .expect("Should be able to create secret key from valid bytes");
                PrivateKey::Ed25519(secret)
            }
            KeyPair::Secp256k1 { secret_key, .. } => PrivateKey::Secp256k1(secret_key.clone()),
        }
    }
    
    /// Sign a message using this key pair
    pub fn sign(&self, message: &[u8]) -> Result<Signature> {
        match self {
            KeyPair::Ed25519(keypair) => {
                let signature = keypair.sign(message);
                Ok(Signature::Ed25519(signature))
            }
            KeyPair::Secp256k1 { .. } => {
                Err(CryptoError::UnsupportedAlgorithm("Secp256k1 signing not implemented".to_string()))
            }
        }
    }
}

impl fmt::Debug for KeyPair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyPair::Ed25519(keypair) => write!(f, "Ed25519KeyPair(public: {})", bs58::encode(keypair.public.as_bytes()).into_string()),
            KeyPair::Secp256k1 { public_key, .. } => write!(f, "Secp256k1KeyPair(public: {})", hex::encode(public_key.to_encoded_point(true).as_bytes())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate(KeyType::Ed25519).unwrap();
        assert_eq!(keypair.key_type(), KeyType::Ed25519);
        
        let public_key = keypair.public_key();
        let private_key = keypair.private_key();
        
        assert_eq!(public_key.key_type(), KeyType::Ed25519);
        assert_eq!(private_key.key_type(), KeyType::Ed25519);
    }
    
    #[test]
    fn test_sign_verify() {
        let keypair = KeyPair::generate(KeyType::Ed25519).unwrap();
        let message = b"test message";
        
        let signature = keypair.sign(message).unwrap();
        let public_key = keypair.public_key();
        
        let result = public_key.verify(message, &signature).unwrap();
        assert!(result);
        
        // Test with wrong message
        let wrong_message = b"wrong message";
        let result = public_key.verify(wrong_message, &signature).unwrap();
        assert!(!result);
    }
} 