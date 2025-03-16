pub mod storage_encryption;

// Re-export commonly used types and functions
pub use storage_encryption::{
    StorageEncryptionService,
    EncryptionMetadata,
    EncryptionError,
    KeyInfoExport,
};

// Utility struct for common crypto operations
pub struct CryptoUtils;

impl CryptoUtils {
    // Compute a SHA-256 hash of the given data
    pub fn sha256_hash(data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        format!("{:x}", result)
    }
    
    // Verify a hash against the given data
    pub fn verify_hash(data: &[u8], expected_hash: &str) -> bool {
        let hash = Self::sha256_hash(data);
        hash == expected_hash
    }
} 