//! Cryptographic primitives for ICN
//! 
//! This crate provides cryptographic functions and types used throughout the ICN project.

pub mod ed25519;
pub mod error;
pub mod key;
pub mod signature;

pub use key::{KeyPair, PublicKey, PrivateKey, KeyType};
pub use signature::Signature;
use error::Result;

/// Sign a message using a key pair
pub fn sign(key_pair: &KeyPair, message: &[u8]) -> Result<Signature> {
    key_pair.sign(message)
}

/// Verify a signature against a message using a public key
pub fn verify(public_key: &PublicKey, message: &[u8], signature: &Signature) -> Result<bool> {
    public_key.verify(message, signature)
}

/// Generate a new key pair of the specified type
pub fn generate_keypair(key_type: KeyType) -> Result<KeyPair> {
    KeyPair::generate(key_type)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sign_verify_roundtrip() {
        let keypair = generate_keypair(KeyType::Ed25519).unwrap();
        let message = b"test message";
        
        let signature = sign(&keypair, message).unwrap();
        let result = verify(&keypair.public_key(), message, &signature).unwrap();
        
        assert!(result);
        
        // Test with wrong message
        let wrong_message = b"wrong message";
        let result = verify(&keypair.public_key(), wrong_message, &signature).unwrap();
        
        assert!(!result);
    }
}
