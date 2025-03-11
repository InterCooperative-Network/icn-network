//! Cryptographic key management for the ICN network
use icn_common::{Error, Result};
use serde::{Serialize, Deserialize};
use std::fmt;
use std::str::FromStr;

/// Supported key types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyType {
    /// Ed25519 signature scheme
    Ed25519,
    /// Secp256k1 signature scheme
    Secp256k1,
    /// X25519 key agreement scheme
    X25519,
}

impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyType::Ed25519 => write!(f, "Ed25519"),
            KeyType::Secp256k1 => write!(f, "Secp256k1"),
            KeyType::X25519 => write!(f, "X25519"),
        }
    }
}

/// Key purpose
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum KeyPurpose {
    /// Signing key
    Signing,
    /// Key agreement (encryption)
    KeyAgreement,
    /// Authentication
    Authentication,
}

impl fmt::Display for KeyPurpose {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyPurpose::Signing => write!(f, "signing"),
            KeyPurpose::KeyAgreement => write!(f, "keyAgreement"),
            KeyPurpose::Authentication => write!(f, "authentication"),
        }
    }
}

impl FromStr for KeyPurpose {
    type Err = Error;
    
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "signing" => Ok(KeyPurpose::Signing),
            "keyAgreement" => Ok(KeyPurpose::KeyAgreement),
            "authentication" => Ok(KeyPurpose::Authentication),
            _ => Err(Error::validation(format!("Unknown key purpose: {}", s))),
        }
    }
}

/// Public key interface
pub trait PublicKey: Send + Sync + std::fmt::Debug {
    /// Get the key type
    fn key_type(&self) -> KeyType;
    
    /// Get the raw key bytes
    fn as_bytes(&self) -> &[u8];
    
    /// Convert key to base58 string
    fn to_base58(&self) -> String;
    
    /// Get the key fingerprint (hash of the key material)
    fn fingerprint(&self) -> String;
    
    /// Clone the key into a boxed trait object
    fn clone_box(&self) -> Box<dyn PublicKey>;
}

impl Clone for Box<dyn PublicKey> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Private key interface
pub trait PrivateKey: Send + Sync + std::fmt::Debug {
    /// Get the key type
    fn key_type(&self) -> KeyType;
    
    /// Get the raw key bytes
    fn as_bytes(&self) -> &[u8];
    
    /// Get the corresponding public key
    fn public_key(&self) -> Box<dyn PublicKey>;
    
    /// Convert key to base58 string (for storage/serialization)
    fn to_base58(&self) -> String;
    
    /// Clone the key into a boxed trait object
    fn clone_box(&self) -> Box<dyn PrivateKey>;
}

impl Clone for Box<dyn PrivateKey> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Key pair interface for managing public/private key pairs
pub trait KeyPair: Send + Sync + std::fmt::Debug {
    /// Get the key type
    fn key_type(&self) -> KeyType;
    
    /// Get the private key
    fn private_key(&self) -> &dyn PrivateKey;
    
    /// Get the public key
    fn public_key(&self) -> &dyn PublicKey;
    
    /// Sign a message with the private key
    fn sign(&self, message: &[u8]) -> Result<Vec<u8>>;
    
    /// Clone the key pair into a boxed trait object
    fn clone_box(&self) -> Box<dyn KeyPair>;
}

impl Clone for Box<dyn KeyPair> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}

/// Key manager for handling storage and retrieval of keys
#[derive(Debug, Default)]
pub struct KeyManager {
    /// Map of key IDs to key pairs
    key_pairs: std::collections::HashMap<String, Box<dyn KeyPair>>,
}

impl KeyManager {
    /// Create a new key manager
    pub fn new() -> Self {
        Self {
            key_pairs: std::collections::HashMap::new(),
        }
    }
    
    /// Add a key pair to the manager
    pub fn add_key_pair(&mut self, id: &str, key_pair: Box<dyn KeyPair>) {
        self.key_pairs.insert(id.to_string(), key_pair);
    }
    
    /// Get a key pair by ID
    pub fn get_key_pair(&self, id: &str) -> Option<&dyn KeyPair> {
        self.key_pairs.get(id).map(|kp| kp.as_ref())
    }
    
    /// Generate a new key pair of the specified type
    pub fn generate_key_pair(&mut self, id: &str, key_type: KeyType) -> Result<&dyn KeyPair> {
        let key_pair = match key_type {
            #[cfg(feature = "ed25519")]
            KeyType::Ed25519 => {
                crate::ed25519::Ed25519KeyPair::generate()?
            }
            #[cfg(feature = "secp256k1")]
            KeyType::Secp256k1 => {
                crate::secp256k1::Secp256k1KeyPair::generate()?
            }
            _ => return Err(Error::configuration(format!(
                "Key type {:?} is not supported or not enabled", key_type
            ))),
        };
        
        self.add_key_pair(id, Box::new(key_pair));
        Ok(self.get_key_pair(id).unwrap())
    }
    
    /// Remove a key pair
    pub fn remove_key_pair(&mut self, id: &str) -> bool {
        self.key_pairs.remove(id).is_some()
    }
    
    /// List all key IDs
    pub fn list_keys(&self) -> Vec<String> {
        self.key_pairs.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    // Most key-related tests are in the algorithm-specific modules
    #[test]
    fn test_key_purpose_display() {
        assert_eq!(KeyPurpose::Signing.to_string(), "signing");
        assert_eq!(KeyPurpose::KeyAgreement.to_string(), "keyAgreement");
        assert_eq!(KeyPurpose::Authentication.to_string(), "authentication");
    }
    
    #[test]
    fn test_key_purpose_from_str() {
        assert_eq!(KeyPurpose::from_str("signing").unwrap(), KeyPurpose::Signing);
        assert_eq!(KeyPurpose::from_str("keyAgreement").unwrap(), KeyPurpose::KeyAgreement);
        assert_eq!(KeyPurpose::from_str("authentication").unwrap(), KeyPurpose::Authentication);
        assert!(KeyPurpose::from_str("unknown").is_err());
    }
    
    // The key manager tests will be covered in integration tests
}