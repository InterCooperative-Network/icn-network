//! Hashing utilities for the ICN network
use icn_common::{Error, Result};
use sha2::{Sha256, Sha512, Digest};
use std::fmt;

/// Supported hash algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    /// SHA-256 hash algorithm
    Sha256,
    /// SHA-512 hash algorithm
    Sha512,
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HashAlgorithm::Sha256 => write!(f, "SHA-256"),
            HashAlgorithm::Sha512 => write!(f, "SHA-512"),
        }
    }
}

/// A cryptographic hash value
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hash {
    /// The hash algorithm used
    pub algorithm: HashAlgorithm,
    /// The hash value as bytes
    pub value: Vec<u8>,
}

impl Hash {
    /// Create a new hash value
    pub fn new(algorithm: HashAlgorithm, value: Vec<u8>) -> Self {
        Self { algorithm, value }
    }
    
    /// Get hash value as hexadecimal string
    pub fn to_hex(&self) -> String {
        self.value.iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
    
    /// Create a hash from hexadecimal string representation
    pub fn from_hex(algorithm: HashAlgorithm, hex: &str) -> Result<Self> {
        if hex.len() % 2 != 0 {
            return Err(Error::validation("Invalid hex string length for hash"));
        }
        
        let expected_len = match algorithm {
            HashAlgorithm::Sha256 => 64, // 32 bytes * 2 chars per byte
            HashAlgorithm::Sha512 => 128, // 64 bytes * 2 chars per byte
        };
        
        if hex.len() != expected_len {
            return Err(Error::validation(
                format!("Invalid length for {} hash: expected {} hex chars, got {}", 
                      algorithm, expected_len, hex.len())
            ));
        }
        
        let mut value = Vec::with_capacity(hex.len() / 2);
        for i in (0..hex.len()).step_by(2) {
            let byte = u8::from_str_radix(&hex[i..i+2], 16)
                .map_err(|_| Error::validation("Invalid hex character in hash"))?;
            value.push(byte);
        }
        
        Ok(Self::new(algorithm, value))
    }
}

/// Hasher trait for creating cryptographic hashes
pub trait Hasher {
    /// The hash algorithm used by this hasher
    fn algorithm(&self) -> HashAlgorithm;
    
    /// Update the hash state with additional data
    fn update(&mut self, data: &[u8]);
    
    /// Finalize the hash computation and return the hash value
    fn finalize(self) -> Hash;
}

/// SHA-256 hasher implementation
pub struct Sha256Hasher {
    hasher: Sha256,
}

impl Sha256Hasher {
    /// Create a new SHA-256 hasher
    pub fn new() -> Self {
        Self { hasher: Sha256::new() }
    }
}

impl Default for Sha256Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Sha256Hasher {
    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Sha256
    }
    
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }
    
    fn finalize(self) -> Hash {
        Hash::new(HashAlgorithm::Sha256, self.hasher.finalize().to_vec())
    }
}

/// SHA-512 hasher implementation
pub struct Sha512Hasher {
    hasher: Sha512,
}

impl Sha512Hasher {
    /// Create a new SHA-512 hasher
    pub fn new() -> Self {
        Self { hasher: Sha512::new() }
    }
}

impl Default for Sha512Hasher {
    fn default() -> Self {
        Self::new()
    }
}

impl Hasher for Sha512Hasher {
    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Sha512
    }
    
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }
    
    fn finalize(self) -> Hash {
        Hash::new(HashAlgorithm::Sha512, self.hasher.finalize().to_vec())
    }
}

/// Convenience function to create a SHA-256 hash of data
pub fn sha256(data: &[u8]) -> Hash {
    let mut hasher = Sha256Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

/// Convenience function to create a SHA-512 hash of data
pub fn sha512(data: &[u8]) -> Hash {
    let mut hasher = Sha512Hasher::new();
    hasher.update(data);
    hasher.finalize()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256(data);
        
        assert_eq!(hash.algorithm, HashAlgorithm::Sha256);
        assert_eq!(
            hash.to_hex(),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }
    
    #[test]
    fn test_sha512() {
        let data = b"hello world";
        let hash = sha512(data);
        
        assert_eq!(hash.algorithm, HashAlgorithm::Sha512);
        assert_eq!(
            hash.to_hex(),
            "309ecc489c12d6eb4cc40f50c902f2b4d0ed77ee511a7c7a9bcd3ca86d4cd86f989dd35bc5ff499670da34255b45b0cfd830e81f605dcf7dc5542e93ae9cd76f"
        );
    }
    
    #[test]
    fn test_incremental_hash() {
        let mut hasher = Sha256Hasher::new();
        hasher.update(b"hello ");
        hasher.update(b"world");
        let hash = hasher.finalize();
        
        let direct_hash = sha256(b"hello world");
        
        assert_eq!(hash.value, direct_hash.value);
    }
    
    #[test]
    fn test_from_hex() {
        let hex = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        let hash = Hash::from_hex(HashAlgorithm::Sha256, hex).unwrap();
        
        assert_eq!(hash.algorithm, HashAlgorithm::Sha256);
        assert_eq!(hash.to_hex(), hex);
        
        // Test invalid hex
        assert!(Hash::from_hex(HashAlgorithm::Sha256, "invalid").is_err());
        assert!(Hash::from_hex(HashAlgorithm::Sha256, "abcd").is_err()); // Too short
    }
}