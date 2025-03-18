use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use ring::aead::{self, Aad, BoundKey, Nonce, NonceSequence, SealingKey, UnboundKey, OpeningKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::crypto::CryptoUtils;

const AES_GCM_KEY_LEN: usize = 32; // 256 bits
const AES_GCM_NONCE_LEN: usize = 12; // 96 bits
const AES_GCM_TAG_LEN: usize = 16; // 128 bits

// A nonce that just wraps a 96-bit array
struct FixedNonceSequence(pub [u8; AES_GCM_NONCE_LEN]);

impl NonceSequence for FixedNonceSequence {
    fn advance(&mut self) -> Result<Nonce, ring::error::Unspecified> {
        Nonce::try_assume_unique_for_key(&self.0)
    }
}

/// Encryption-related errors
#[derive(Debug, Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    
    #[error("Key management error: {0}")]
    KeyManagementError(String),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Invalid parameters: {0}")]
    InvalidParameters(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(e: ring::error::Unspecified) -> Self {
        EncryptionError::Other(format!("Ring library error: {}", e))
    }
}

/// Encryption metadata stored alongside the encrypted data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    pub key_id: String,
    pub algorithm: String,
    pub iv: Vec<u8>,
    pub auth_tag: Option<Vec<u8>>,
    pub version: u8,
}

/// Information about a key that can be exported (no secret material)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub id: String,
    pub algorithm: String,
    pub created_at: u64,
    pub federations: HashSet<String>,
    pub is_active: bool,
}

/// Full key information including secret material
#[derive(Debug, Clone)]
struct KeyData {
    pub info: KeyInfo,
    pub key_material: Vec<u8>,
}

/// The Storage Encryption Service manages encryption for storage data
pub struct StorageEncryptionService {
    // All encryption keys
    keys: RwLock<HashMap<String, KeyData>>,
    // System random source
    rng: SystemRandom,
}

impl StorageEncryptionService {
    /// Create a new encryption service
    pub fn new() -> Self {
        StorageEncryptionService {
            keys: RwLock::new(HashMap::new()),
            rng: SystemRandom::new(),
        }
    }
    
    /// Generate a new encryption key for the given federations
    pub async fn generate_key(&self, federations: Vec<String>) -> Result<String, EncryptionError> {
        // Generate a random key
        let mut key_material = vec![0u8; AES_GCM_KEY_LEN];
        self.rng.fill(&mut key_material)
            .map_err(|_| EncryptionError::KeyManagementError("Failed to generate random key".to_string()))?;
        
        // Create a unique ID for the key
        let mut id_bytes = [0u8; 16];
        self.rng.fill(&mut id_bytes)
            .map_err(|_| EncryptionError::KeyManagementError("Failed to generate key ID".to_string()))?;
        
        let key_id = Self::generate_key_id(&id_bytes)?;
        
        // Create key info
        let info = KeyInfo {
            id: key_id.clone(),
            algorithm: "AES-GCM-256".to_string(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            federations: federations.into_iter().collect(),
            is_active: true,
        };
        
        // Store the key
        let key_data = KeyData {
            info,
            key_material,
        };
        
        let mut keys = self.keys.write().unwrap();
        keys.insert(key_id.clone(), key_data);
        
        Ok(key_id)
    }
    
    /// Check if a federation has access to a key
    pub async fn federation_has_key_access(&self, federation_id: &str, key_id: &str) -> bool {
        let keys = self.keys.read().unwrap();
        
        if let Some(key_data) = keys.get(key_id) {
            key_data.info.federations.contains(federation_id)
        } else {
            false
        }
    }
    
    /// Grant a federation access to a key
    pub async fn grant_federation_key_access(&self, federation_id: &str, key_id: &str) -> Result<(), EncryptionError> {
        let mut keys = self.keys.write().unwrap();
        
        let key_data = keys.get_mut(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
            
        key_data.info.federations.insert(federation_id.to_string());
        
        Ok(())
    }
    
    /// Remove a federation's access to a key
    pub async fn revoke_federation_key_access(&self, federation_id: &str, key_id: &str) -> Result<(), EncryptionError> {
        let mut keys = self.keys.write().unwrap();
        
        let key_data = keys.get_mut(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
            
        key_data.info.federations.remove(federation_id);
        
        Ok(())
    }
    
    /// List all keys a federation has access to
    pub async fn list_federation_keys(&self, federation_id: &str) -> Vec<KeyInfo> {
        let keys = self.keys.read().unwrap();
        
        keys.values()
            .filter(|key_data| key_data.info.federations.contains(federation_id))
            .map(|key_data| key_data.info.clone())
            .collect()
    }
    
    /// Encrypt data using a key
    pub async fn encrypt(
        &self,
        key_id: &str,
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, EncryptionMetadata), EncryptionError> {
        let keys = self.keys.read().unwrap();
        
        let key_data = keys.get(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
        
        // Create a nonce
        let mut nonce_bytes = [0u8; AES_GCM_NONCE_LEN];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|_| EncryptionError::EncryptionFailed("Failed to generate nonce".to_string()))?;
        
        // Create a copy of the plaintext that we can modify (for in-place encryption)
        let mut in_out = plaintext.to_vec();
        
        // Set up encryption
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_data.key_material)
            .map_err(|_| EncryptionError::EncryptionFailed("Invalid key".to_string()))?;
        
        let nonce_sequence = FixedNonceSequence(nonce_bytes);
        let aad = Aad::empty();
        
        let mut sealing_key = SealingKey::new(unbound_key, nonce_sequence);
        
        // Encrypt in place
        sealing_key.seal_in_place_append_tag(aad, &mut in_out)
            .map_err(|_| EncryptionError::EncryptionFailed("Encryption failed".to_string()))?;
        
        // Create metadata
        let metadata = EncryptionMetadata {
            key_id: key_id.to_string(),
            algorithm: key_data.info.algorithm.clone(),
            iv: nonce_bytes.to_vec(),
            auth_tag: None, // Tag is appended to the ciphertext with ring
            version: 1,
        };
        
        Ok((in_out, metadata))
    }
    
    /// Decrypt data using a key
    pub async fn decrypt(
        &self,
        key_id: &str,
        ciphertext: &[u8],
        metadata: &EncryptionMetadata,
    ) -> Result<Vec<u8>, EncryptionError> {
        let keys = self.keys.read().unwrap();
        
        let key_data = keys.get(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
        
        // Verify metadata
        if metadata.key_id != key_id {
            return Err(EncryptionError::InvalidParameters("Key ID mismatch".to_string()));
        }
        
        if metadata.iv.len() != AES_GCM_NONCE_LEN {
            return Err(EncryptionError::InvalidParameters("Invalid nonce length".to_string()));
        }
        
        // Convert IV to fixed array
        let mut nonce_bytes = [0u8; AES_GCM_NONCE_LEN]; 
        nonce_bytes.copy_from_slice(&metadata.iv);
        
        // Create a copy of the ciphertext that we can modify (for in-place decryption)
        let mut in_out = ciphertext.to_vec();
        
        // Set up decryption
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_data.key_material)
            .map_err(|_| EncryptionError::DecryptionFailed("Invalid key".to_string()))?;
            
        let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| EncryptionError::DecryptionFailed("Invalid nonce".to_string()))?;
            
        let aad = Aad::empty();
        
        // Decrypt in place
        let decrypted = ring::aead::open_in_place(unbound_key, nonce, aad, 0, &mut in_out)
            .map_err(|_| EncryptionError::DecryptionFailed("Decryption failed - data may be corrupted".to_string()))?;
        
        Ok(decrypted.to_vec())
    }
    
    // Helper to generate a key ID from bytes
    fn generate_key_id(bytes: &[u8]) -> Result<String, EncryptionError> {
        Ok(format!("key-{}", hex::encode(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_encryption_cycle() {
        let service = StorageEncryptionService::new();
        let federations = vec!["fed1".to_string(), "fed2".to_string()];
        
        // Generate a key
        let key_id = service.generate_key(federations).await.unwrap();
        
        // Encrypt data
        let plaintext = b"This is a secret message";
        let (ciphertext, metadata) = service.encrypt(&key_id, plaintext).await.unwrap();
        
        // Ciphertext should be different
        assert_ne!(&ciphertext[..], plaintext);
        
        // Decrypt data
        let decrypted = service.decrypt(&key_id, &ciphertext, &metadata).await.unwrap();
        
        // Decrypted data should match original
        assert_eq!(&decrypted[..], plaintext);
    }
    
    #[tokio::test]
    async fn test_federation_access() {
        let service = StorageEncryptionService::new();
        let federations = vec!["fed1".to_string()];
        
        // Generate a key for fed1
        let key_id = service.generate_key(federations).await.unwrap();
        
        // Check access
        assert!(service.federation_has_key_access("fed1", &key_id).await);
        assert!(!service.federation_has_key_access("fed2", &key_id).await);
        
        // Grant access to fed2
        service.grant_federation_key_access("fed2", &key_id).await.unwrap();
        
        // Check access again
        assert!(service.federation_has_key_access("fed2", &key_id).await);
        
        // Revoke access from fed2
        service.revoke_federation_key_access("fed2", &key_id).await.unwrap();
        
        // Check access again
        assert!(!service.federation_has_key_access("fed2", &key_id).await);
    }
    
    #[tokio::test]
    async fn test_list_federation_keys() {
        let service = StorageEncryptionService::new();
        
        // Generate keys for different federations
        let key_id1 = service.generate_key(vec!["fed1".to_string()]).await.unwrap();
        let key_id2 = service.generate_key(vec!["fed1".to_string(), "fed2".to_string()]).await.unwrap();
        let key_id3 = service.generate_key(vec!["fed2".to_string()]).await.unwrap();
        
        // List keys for fed1
        let fed1_keys = service.list_federation_keys("fed1").await;
        assert_eq!(fed1_keys.len(), 2);
        assert!(fed1_keys.iter().any(|k| k.id == key_id1));
        assert!(fed1_keys.iter().any(|k| k.id == key_id2));
        
        // List keys for fed2
        let fed2_keys = service.list_federation_keys("fed2").await;
        assert_eq!(fed2_keys.len(), 2);
        assert!(fed2_keys.iter().any(|k| k.id == key_id2));
        assert!(fed2_keys.iter().any(|k| k.id == key_id3));
        
        // List keys for fed3 (should be empty)
        let fed3_keys = service.list_federation_keys("fed3").await;
        assert_eq!(fed3_keys.len(), 0);
    }
} 