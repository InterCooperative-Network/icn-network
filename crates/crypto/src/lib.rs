pub mod storage_encryption;

use std::sync::Arc;
use rand::{rngs::OsRng, RngCore};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use sha2::{Sha256, Digest};
use thiserror::Error;
use std::fmt;

// Re-export storage encryption types
pub use storage_encryption::{StorageEncryptionService, EncryptionMetadata, EncryptionError};

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("Key generation error: {0}")]
    KeyGenerationError(String),
    
    #[error("Signing error: {0}")]
    SigningError(String),
    
    #[error("Verification error: {0}")]
    VerificationError(String),
    
    #[error("Encryption error: {0}")]
    EncryptionError(String),
    
    #[error("Decryption error: {0}")]
    DecryptionError(String),
    
    #[error("Hashing error: {0}")]
    HashingError(String),
    
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    
    #[error("Other crypto error: {0}")]
    Other(String),
}

pub type CryptoResult<T> = Result<T, CryptoError>;

/// Hash type for cryptographic operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash(pub Vec<u8>);

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(&self.0))
    }
}

/// General cryptographic utilities for the ICN network
pub struct CryptoUtils;

impl CryptoUtils {
    /// Generate a random ed25519 keypair for signing
    pub fn generate_keypair() -> CryptoResult<Keypair> {
        let mut csprng = OsRng {};
        let keypair = Keypair::generate(&mut csprng);
        Ok(keypair)
    }
    
    /// Sign a message using ed25519
    pub fn sign(keypair: &Keypair, message: &[u8]) -> CryptoResult<Signature> {
        Ok(keypair.sign(message))
    }
    
    /// Verify a signature using ed25519
    pub fn verify(public_key: &PublicKey, message: &[u8], signature: &Signature) -> CryptoResult<bool> {
        match public_key.verify(message, signature) {
            Ok(()) => Ok(true),
            Err(e) => Err(CryptoError::VerificationError(e.to_string())),
        }
    }
    
    /// Generate a random seed
    pub fn generate_random_seed(length: usize) -> CryptoResult<Vec<u8>> {
        let mut bytes = vec![0u8; length];
        OsRng.fill_bytes(&mut bytes);
        Ok(bytes)
    }
    
    /// Generate a secure random key
    pub fn generate_key(length: usize) -> CryptoResult<Vec<u8>> {
        Self::generate_random_seed(length)
    }
    
    /// Calculate SHA-256 hash of data
    pub fn sha256(data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Hash(hasher.finalize().to_vec())
    }
    
    /// Compare two digests in constant time
    pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
        if a.len() != b.len() {
            return false;
        }
        
        let mut result = 0;
        for (x, y) in a.iter().zip(b.iter()) {
            result |= x ^ y;
        }
        
        result == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_keypair() {
        let keypair = CryptoUtils::generate_keypair().unwrap();
        assert_eq!(keypair.public.as_bytes().len(), 32);
        assert_eq!(keypair.secret.as_bytes().len(), 32);
    }
    
    #[test]
    fn test_sign_verify() {
        let keypair = CryptoUtils::generate_keypair().unwrap();
        let message = b"test message";
        
        let signature = CryptoUtils::sign(&keypair, message).unwrap();
        let result = CryptoUtils::verify(&keypair.public, message, &signature).unwrap();
        
        assert!(result);
    }
    
    #[test]
    fn test_sign_verify_wrong_message() {
        let keypair = CryptoUtils::generate_keypair().unwrap();
        let message = b"test message";
        let wrong_message = b"wrong message";
        
        let signature = CryptoUtils::sign(&keypair, message).unwrap();
        let result = CryptoUtils::verify(&keypair.public, wrong_message, &signature);
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_sha256() {
        let data = b"test data";
        let hash = CryptoUtils::sha256(data);
        
        // Known SHA-256 hash of "test data"
        let expected_hex = "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9";
        let expected = hex::decode(expected_hex).unwrap();
        
        assert_eq!(hash.0, expected);
    }
    
    #[test]
    fn test_constant_time_eq() {
        let a = vec![1, 2, 3, 4];
        let b = vec![1, 2, 3, 4];
        let c = vec![1, 2, 3, 5];
        
        assert!(CryptoUtils::constant_time_eq(&a, &b));
        assert!(!CryptoUtils::constant_time_eq(&a, &c));
    }
}
