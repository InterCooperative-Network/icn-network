//! Ed25519 cryptographic primitives

use ed25519_dalek::{Keypair, PublicKey, SecretKey, Signature};
use rand::rngs::OsRng;

use crate::error::{CryptoError, Result};

/// Generate a new Ed25519 keypair
pub fn generate_keypair() -> Result<Keypair> {
    let mut csprng = OsRng;
    let keypair = Keypair::generate(&mut csprng);
    Ok(keypair)
}

/// Create a public key from bytes
pub fn public_key_from_bytes(bytes: &[u8]) -> Result<PublicKey> {
    PublicKey::from_bytes(bytes)
        .map_err(|e| CryptoError::InvalidKey(format!("Invalid Ed25519 public key: {}", e)))
}

/// Create a secret key from bytes
pub fn secret_key_from_bytes(bytes: &[u8]) -> Result<SecretKey> {
    SecretKey::from_bytes(bytes)
        .map_err(|e| CryptoError::InvalidKey(format!("Invalid Ed25519 secret key: {}", e)))
}

/// Create a keypair from a secret key
pub fn keypair_from_secret_key(secret_key: SecretKey) -> Result<Keypair> {
    let public_key = PublicKey::from(&secret_key);
    Ok(Keypair {
        public: public_key,
        secret: secret_key,
    })
}

/// Create a signature from bytes
pub fn signature_from_bytes(bytes: &[u8]) -> Result<Signature> {
    Signature::from_bytes(bytes)
        .map_err(|e| CryptoError::InvalidSignature(format!("Invalid Ed25519 signature: {}", e)))
}

/// Convert an Ed25519 public key to base58 string
pub fn public_key_to_base58(public_key: &PublicKey) -> String {
    bs58::encode(public_key.as_bytes()).into_string()
}

/// Convert a base58 string to an Ed25519 public key
pub fn public_key_from_base58(encoded: &str) -> Result<PublicKey> {
    let bytes = bs58::decode(encoded)
        .into_vec()
        .map_err(|e| CryptoError::InvalidKey(format!("Invalid base58 encoding: {}", e)))?;
    
    public_key_from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;
    use ed25519_dalek::Verifier;
    
    #[test]
    fn test_generate_keypair() {
        let keypair = generate_keypair().unwrap();
        assert_ne!(keypair.public.as_bytes().len(), 0);
        assert_ne!(keypair.secret.as_bytes().len(), 0);
    }
    
    #[test]
    fn test_sign_verify() {
        let keypair = generate_keypair().unwrap();
        let message = b"test message";
        
        let signature = keypair.sign(message);
        
        assert!(keypair.public.verify(message, &signature).is_ok());
        
        // Negative test
        let wrong_message = b"wrong message";
        assert!(keypair.public.verify(wrong_message, &signature).is_err());
    }
    
    #[test]
    fn test_base58_conversion() {
        let keypair = generate_keypair().unwrap();
        let base58_pubkey = public_key_to_base58(&keypair.public);
        
        let recovered_pubkey = public_key_from_base58(&base58_pubkey).unwrap();
        
        assert_eq!(keypair.public.as_bytes(), recovered_pubkey.as_bytes());
    }
}