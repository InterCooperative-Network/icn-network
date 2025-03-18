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
    
    /// Check if a federation has access to a specific key
    pub async fn federation_has_key_access(&self, federation_id: &str, key_id: &str) -> Result<bool, EncryptionError> {
        let keys = self.keys.read().map_err(|e| EncryptionError::Other(format!("Lock error: {}", e)))?;
        
        match keys.get(key_id) {
            Some(key_data) => Ok(key_data.info.federations.contains(federation_id)),
            None => Err(EncryptionError::KeyNotFound(key_id.to_string())),
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
    
    /// Encrypt data using the specified key
    pub async fn encrypt(
        &self,
        data: &[u8],
        key_id: &str,
    ) -> Result<(Vec<u8>, HashMap<String, String>), EncryptionError> {
        // Get the key
        let keys = self.keys.read().map_err(|e| EncryptionError::Other(format!("Lock error: {}", e)))?;
        let key_data = keys.get(key_id).ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
        
        // Generate a random IV
        let mut iv = [0u8; AES_GCM_NONCE_LEN];
        self.rng.fill(&mut iv).map_err(|e| EncryptionError::EncryptionFailed(format!("Failed to generate IV: {:?}", e)))?;
        
        // Create a nonce sequence
        let nonce_sequence = FixedNonceSequence(iv);
        
        // Get the unbound key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_data.key_material)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("Failed to create key: {:?}", e)))?;
            
        // Create AAD (empty for now)
        let aad = Aad::empty();
        
        // Copy the plaintext to a mutable buffer for in-place encryption
        let mut in_out = data.to_vec();
        
        // Reserve space for the authentication tag
        in_out.extend_from_slice(&[0u8; AES_GCM_TAG_LEN]);
        
        // Perform the encryption
        let sealing_key = aead::SealingKey::new(unbound_key, nonce_sequence);
        let encrypted_len = sealing_key.seal_in_place_append_tag(aad, &mut in_out)
            .map_err(|e| EncryptionError::EncryptionFailed(format!("Encryption failed: {:?}", e)))?;
            
        // Resize to the actual encrypted length
        in_out.truncate(encrypted_len);
        
        // Create metadata
        let mut metadata = HashMap::new();
        metadata.insert("key_id".to_string(), key_id.to_string());
        metadata.insert("algorithm".to_string(), "AES-256-GCM".to_string());
        metadata.insert("iv".to_string(), hex::encode(&iv));
        metadata.insert("version".to_string(), "1".to_string());
        
        Ok((in_out, metadata))
    }
    
    /// Decrypt data using the specified key
    pub async fn decrypt(
        &self,
        data: &[u8],
        metadata: &HashMap<String, String>,
    ) -> Result<Vec<u8>, EncryptionError> {
        // Extract key_id from metadata
        let key_id = metadata.get("key_id").ok_or_else(|| 
            EncryptionError::InvalidParameters("Missing key_id in metadata".to_string()))?;
            
        // Extract IV from metadata
        let iv_hex = metadata.get("iv").ok_or_else(|| 
            EncryptionError::InvalidParameters("Missing IV in metadata".to_string()))?;
        let iv = hex::decode(iv_hex)
            .map_err(|e| EncryptionError::InvalidParameters(format!("Invalid IV format: {}", e)))?;
            
        // Get the key
        let keys = self.keys.read().map_err(|e| EncryptionError::Other(format!("Lock error: {}", e)))?;
        let key_data = keys.get(key_id).ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
        
        // Create a nonce from the IV
        let nonce_sequence = FixedNonceSequence(
            iv.try_into().map_err(|_| EncryptionError::InvalidParameters("Invalid IV length".to_string()))?
        );
        
        // Get the unbound key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_data.key_material)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Failed to create key: {:?}", e)))?;
            
        // Create AAD (empty for now)
        let aad = Aad::empty();
        
        // Copy the ciphertext to a mutable buffer for in-place decryption
        let mut in_out = data.to_vec();
        
        // Perform the decryption
        let opening_key = aead::OpeningKey::new(unbound_key, nonce_sequence);
        let decrypted_len = opening_key.open_in_place(aad, &mut in_out)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Decryption failed: {:?}", e)))?
            .len();
            
        // Resize to the actual decrypted length
        in_out.truncate(decrypted_len);
        
        Ok(in_out)
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
        let (ciphertext, metadata) = service.encrypt(plaintext, &key_id).await.unwrap();
        
        // Ciphertext should be different
        assert_ne!(&ciphertext[..], plaintext);
        
        // Decrypt data
        let decrypted = service.decrypt(&ciphertext, &metadata).await.unwrap();
        
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
        assert!(service.federation_has_key_access("fed1", &key_id).await.unwrap());
        assert!(!service.federation_has_key_access("fed2", &key_id).await.unwrap());
        
        // Grant access to fed2
        service.grant_federation_key_access("fed2", &key_id).await.unwrap();
        
        // Check access again
        assert!(service.federation_has_key_access("fed2", &key_id).await.unwrap());
        
        // Revoke access from fed2
        service.revoke_federation_key_access("fed2", &key_id).await.unwrap();
        
        // Check access again
        assert!(!service.federation_has_key_access("fed2", &key_id).await.unwrap());
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