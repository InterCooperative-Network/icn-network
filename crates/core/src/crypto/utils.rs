use rand::{rngs::OsRng, RngCore};
use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature, Signer, Verifier};
use sha2::{Sha256, Digest};
use crate::crypto::{Hash, CryptoError, CryptoResult};

/// Utility functions for cryptographic operations
pub struct CryptoUtils;

impl CryptoUtils {
    /// Generate a new Ed25519 keypair
    pub fn generate_keypair() -> CryptoResult<Keypair> {
        let mut csprng = OsRng;
        let keypair = Keypair::generate(&mut csprng);
        Ok(keypair)
    }
    
    /// Sign a message with a keypair
    pub fn sign(keypair: &Keypair, message: &[u8]) -> CryptoResult<Signature> {
        Ok(keypair.sign(message))
    }
    
    /// Verify a signature against a message and public key
    pub fn verify(public_key: &PublicKey, message: &[u8], signature: &Signature) -> CryptoResult<bool> {
        match public_key.verify(message, signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }
    
    /// Generate a random seed of specified length
    pub fn generate_random_seed(length: usize) -> CryptoResult<Vec<u8>> {
        let mut seed = vec![0u8; length];
        OsRng.fill_bytes(&mut seed);
        Ok(seed)
    }
    
    /// Generate a cryptographic key of specified length
    pub fn generate_key(length: usize) -> CryptoResult<Vec<u8>> {
        Self::generate_random_seed(length)
    }
    
    /// Compute the SHA-256 hash of data
    pub fn sha256(data: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hash = hasher.finalize();
        Hash(hash.to_vec())
    }
    
    /// Compare two byte arrays in constant time to prevent timing attacks
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