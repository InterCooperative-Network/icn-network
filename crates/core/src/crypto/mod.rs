//! Cryptography module for ICN
//!
//! This module provides cryptographic primitives for secure operations
//! in the InterCooperative Network.

use std::fmt;
use thiserror::Error;
use ring::digest;
use ring::signature::{self, Ed25519KeyPair, KeyPair};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};

/// Error types for cryptographic operations
#[derive(Error, Debug)]
pub enum CryptoError {
    /// Error during key generation
    #[error("Key generation error: {0}")]
    KeyGenError(String),
    
    /// Error during signing
    #[error("Signing error: {0}")]
    SigningError(String),
    
    /// Error during verification
    #[error("Verification error: {0}")]
    VerificationError(String),
    
    /// Error during hashing
    #[error("Hashing error: {0}")]
    HashingError(String),
    
    /// Error during serialization
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for cryptographic operations
pub type CryptoResult<T> = Result<T, CryptoError>;

/// A hash value
#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub Vec<u8>);

impl Hash {
    /// Create a new Hash from bytes
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
    
    /// Get the bytes of the hash
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert hash to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
    
    /// Create a hash from a hex string
    pub fn from_hex(hex_str: &str) -> CryptoResult<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| CryptoError::HashingError(format!("Invalid hex: {}", e)))?;
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", self.to_hex())
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// A digital signature
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Signature(pub Vec<u8>);

impl Signature {
    /// Create a new Signature from bytes
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
    
    /// Get the bytes of the signature
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert signature to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(&self.0)
    }
    
    /// Create a signature from a hex string
    pub fn from_hex(hex_str: &str) -> CryptoResult<Self> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| CryptoError::SigningError(format!("Invalid hex: {}", e)))?;
        Ok(Self(bytes))
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Signature({})", self.to_hex())
    }
}

impl fmt::Display for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// A Key Pair for digital signatures
pub struct KeyPairWrapper {
    /// The internal Ed25519 key pair
    key_pair: Ed25519KeyPair,
}

impl KeyPairWrapper {
    /// Generate a new random key pair
    pub fn generate() -> CryptoResult<Self> {
        let rng = ring::rand::SystemRandom::new();
        let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|e| CryptoError::KeyGenError(format!("Failed to generate key pair: {:?}", e)))?;
        
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref())
            .map_err(|e| CryptoError::KeyGenError(format!("Failed to parse key pair: {:?}", e)))?;
        
        Ok(Self { key_pair })
    }
    
    /// Create a KeyPair from PKCS#8 encoded bytes
    pub fn from_pkcs8(pkcs8_bytes: &[u8]) -> CryptoResult<Self> {
        let key_pair = Ed25519KeyPair::from_pkcs8(pkcs8_bytes)
            .map_err(|e| CryptoError::KeyGenError(format!("Failed to parse key pair: {:?}", e)))?;
        
        Ok(Self { key_pair })
    }
    
    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> &[u8] {
        self.key_pair.public_key().as_ref()
    }
    
    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        let signature = self.key_pair.sign(message);
        Signature(signature.as_ref().to_vec())
    }
}

/// Verify a signature with a public key
pub fn verify_signature(public_key: &[u8], message: &[u8], signature: &Signature) -> CryptoResult<()> {
    let public_key = signature::UnparsedPublicKey::new(&signature::ED25519, public_key);
    
    public_key.verify(message, signature.as_bytes())
        .map_err(|e| CryptoError::VerificationError(format!("Signature verification failed: {:?}", e)))?;
    
    Ok(())
}

/// Calculate SHA-256 hash of data
pub fn sha256(data: &[u8]) -> Hash {
    let digest = digest::digest(&digest::SHA256, data);
    Hash(digest.as_ref().to_vec())
}

pub mod merkle;
pub mod identity;

// Re-exports
pub use merkle::MerkleTree;
pub use identity::IdentityKeyPair; 