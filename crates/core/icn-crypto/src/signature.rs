//! Digital signature functionality for ICN
use icn_common::{Error, Result};
use serde::{Serialize, Deserialize};
use crate::keys::KeyType;
use std::fmt;

/// Signature algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SignatureAlgorithm {
    /// Ed25519 signature algorithm
    Ed25519,
    /// Secp256k1 signature algorithm with ECDSA
    Secp256k1,
}

impl fmt::Display for SignatureAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignatureAlgorithm::Ed25519 => write!(f, "Ed25519"),
            SignatureAlgorithm::Secp256k1 => write!(f, "Secp256k1"),
        }
    }
}

/// Map KeyType to corresponding SignatureAlgorithm
impl From<KeyType> for SignatureAlgorithm {
    fn from(key_type: KeyType) -> Self {
        match key_type {
            KeyType::Ed25519 => SignatureAlgorithm::Ed25519,
            KeyType::Secp256k1 => SignatureAlgorithm::Secp256k1,
            _ => panic!("Key type {:?} does not support signatures", key_type),
        }
    }
}

/// Digital signature with metadata
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// The signature algorithm used
    pub algorithm: SignatureAlgorithm,
    /// The raw signature bytes
    pub value: Vec<u8>,
    /// The key identifier that created this signature
    pub key_id: Option<String>,
}

impl Signature {
    /// Create a new signature
    pub fn new(algorithm: SignatureAlgorithm, value: Vec<u8>) -> Self {
        Self {
            algorithm,
            value,
            key_id: None,
        }
    }
    
    /// Create a new signature with key ID
    pub fn with_key_id(algorithm: SignatureAlgorithm, value: Vec<u8>, key_id: String) -> Self {
        Self {
            algorithm,
            value,
            key_id: Some(key_id),
        }
    }
    
    /// Signature value as base64 string
    pub fn to_base64(&self) -> String {
        base64::encode(&self.value)
    }
    
    /// Create a signature from base64-encoded string
    pub fn from_base64(algorithm: SignatureAlgorithm, encoded: &str) -> Result<Self> {
        let value = base64::decode(encoded)
            .map_err(|e| Error::validation(format!("Invalid base64 string: {}", e)))?;
            
        Ok(Self::new(algorithm, value))
    }
}

/// Trait for creating signatures
pub trait Signer {
    /// Get the signature algorithm used by this signer
    fn algorithm(&self) -> SignatureAlgorithm;
    
    /// Sign data and return the signature
    fn sign(&self, data: &[u8]) -> Result<Signature>;
}

/// Trait for verifying signatures
pub trait Verifier {
    /// Get the signature algorithm supported by this verifier
    fn algorithm(&self) -> SignatureAlgorithm;
    
    /// Verify a signature against data
    fn verify(&self, data: &[u8], signature: &Signature) -> Result<bool>;
}

/// Multi-signature representing multiple signatures over the same data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSignature {
    /// Vector of signatures
    pub signatures: Vec<Signature>,
    /// Threshold required for validity (if None, all signatures must verify)
    pub threshold: Option<usize>,
}

impl MultiSignature {
    /// Create a new multi-signature
    pub fn new() -> Self {
        Self {
            signatures: Vec::new(),
            threshold: None,
        }
    }
    
    /// Add a signature to the multi-signature
    pub fn add(&mut self, signature: Signature) {
        self.signatures.push(signature);
    }
    
    /// Set the threshold for verification
    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = Some(threshold);
        self
    }
    
    /// Get the number of signatures
    pub fn len(&self) -> usize {
        self.signatures.len()
    }
    
    /// Check if there are no signatures
    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }
}

impl Default for MultiSignature {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_signature_base64() {
        let sig_data = vec![1, 2, 3, 4, 5];
        let sig = Signature::new(SignatureAlgorithm::Ed25519, sig_data.clone());
        
        let base64_str = sig.to_base64();
        let decoded_sig = Signature::from_base64(SignatureAlgorithm::Ed25519, &base64_str).unwrap();
        
        assert_eq!(decoded_sig.value, sig_data);
        assert_eq!(decoded_sig.algorithm, SignatureAlgorithm::Ed25519);
    }
    
    #[test]
    fn test_multi_signature() {
        let sig1 = Signature::new(SignatureAlgorithm::Ed25519, vec![1, 2, 3]);
        let sig2 = Signature::new(SignatureAlgorithm::Secp256k1, vec![4, 5, 6]);
        
        let mut multi_sig = MultiSignature::new();
        multi_sig.add(sig1);
        multi_sig.add(sig2);
        
        assert_eq!(multi_sig.len(), 2);
        assert!(!multi_sig.is_empty());
        
        let threshold_sig = multi_sig.with_threshold(1);
        assert_eq!(threshold_sig.threshold, Some(1));
    }
}