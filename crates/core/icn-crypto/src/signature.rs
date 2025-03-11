//! Signature types for the ICN crypto system

use std::fmt;
use crate::error::Result;

/// Signature for cryptographic operations
pub enum Signature {
    /// Ed25519 signature
    Ed25519(ed25519_dalek::Signature),
    
    /// Secp256k1 signature
    Secp256k1(Vec<u8>),
}

impl Signature {
    /// Create a new signature from bytes
    /// 
    /// We don't know the type of signature, so we try to parse it as Ed25519 first
    pub fn new_from_bytes(bytes: Vec<u8>) -> Self {
        // Try to parse as Ed25519 signature
        if bytes.len() == ed25519_dalek::SIGNATURE_LENGTH {
            if let Ok(sig) = ed25519_dalek::Signature::from_bytes(&bytes) {
                return Signature::Ed25519(sig);
            }
        }
        
        // Fallback to Secp256k1 (or unknown)
        Signature::Secp256k1(bytes)
    }
    
    /// Export this signature as bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Signature::Ed25519(sig) => sig.to_bytes().to_vec(),
            Signature::Secp256k1(bytes) => bytes.clone(),
        }
    }
}

impl fmt::Debug for Signature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Signature::Ed25519(sig) => write!(f, "Ed25519Signature({})", bs58::encode(sig.to_bytes()).into_string()),
            Signature::Secp256k1(bytes) => write!(f, "Secp256k1Signature({})", hex::encode(bytes)),
        }
    }
}

/// A trait for signing messages
pub trait Signer {
    /// Sign a message
    fn sign(&self, message: &[u8]) -> Result<Signature>;
}

/// A trait for verifying signatures
pub trait Verifier {
    /// Verify a signature against a message
    fn verify(&self, message: &[u8], signature: &Signature) -> Result<bool>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Keypair, Signer as DalekSigner};
    
    #[test]
    fn test_signature_basics() {
        // Generate a keypair for testing
        let mut csprng = rand::rngs::OsRng;
        let keypair = Keypair::generate(&mut csprng);
        
        // Sign a message
        let message = b"test message";
        let dalek_sig = keypair.sign(message);
        
        // Convert to our signature type
        let signature = Signature::Ed25519(dalek_sig);
        let bytes = signature.to_bytes();
        
        // Round trip through bytes
        let recovered = Signature::new_from_bytes(bytes);
        
        // Verify it's the same signature
        match recovered {
            Signature::Ed25519(sig) => assert_eq!(sig.to_bytes(), dalek_sig.to_bytes()),
            _ => panic!("Wrong signature type after round trip"),
        }
    }
}