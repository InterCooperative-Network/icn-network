//! Cryptographic primitives for the Intercooperative Network
//!
//! This crate provides cryptographic functionality including:
//! - Key generation and management
//! - Digital signatures
//! - Hashing utilities
//! - Encryption/decryption

mod hash;
mod keys;
mod signature;
#[cfg(feature = "ed25519")]
mod ed25519;
#[cfg(feature = "secp256k1")]
mod secp256k1;

// Re-exports
pub use hash::{Hash, HashAlgorithm, Hasher};
pub use keys::{KeyPair, PublicKey, PrivateKey};
pub use signature::{Signature, Signer, Verifier};

/// Create a new key pair using the default algorithm
pub fn generate_keypair() -> icn_common::Result<Box<dyn KeyPair>> {
    #[cfg(feature = "ed25519")]
    {
        Ok(Box::new(ed25519::Ed25519KeyPair::generate()?))
    }
    #[cfg(all(feature = "secp256k1", not(feature = "ed25519")))]
    {
        Ok(Box::new(secp256k1::Secp256k1KeyPair::generate()?))
    }
    #[cfg(not(any(feature = "ed25519", feature = "secp256k1")))]
    {
        Err(icn_common::Error::configuration(
            "No signature algorithm enabled. Enable either 'ed25519' or 'secp256k1' feature."
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_generate_and_verify() {
        let keypair = generate_keypair().expect("Failed to generate keypair");
        let message = b"test message";
        
        let signature = keypair.sign(message).expect("Failed to sign message");
        let verified = keypair.public_key().verify(message, &signature)
            .expect("Failed to verify signature");
        
        assert!(verified);
    }
}
