use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use ring::aead::{self, Aad, BoundKey, Nonce, NonceSequence, SealingKey, UnboundKey, OpeningKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::CryptoUtils;

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
        let mut sealing_key = SealingKey::new(unbound_key, nonce_sequence);
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
        if iv.len() != AES_GCM_NONCE_LEN {
            return Err(EncryptionError::InvalidParameters(format!("Invalid IV length: {} (expected {})", iv.len(), AES_GCM_NONCE_LEN)));
        }
        
        let mut iv_array = [0u8; AES_GCM_NONCE_LEN];
        iv_array.copy_from_slice(&iv);
        
        let nonce_sequence = FixedNonceSequence(iv_array);
        
        // Get the unbound key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_data.key_material)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Failed to create key: {:?}", e)))?;
        
        // Create AAD (empty for now)
        let aad = Aad::empty();
        
        // Copy the ciphertext to a mutable buffer for in-place decryption
        let mut in_out = data.to_vec();
        
        // Perform the decryption
        let mut opening_key = aead::OpeningKey::new(unbound_key, nonce_sequence);
        let decrypted_len = opening_key.open_in_place(aad, &mut in_out)
            .map_err(|e| EncryptionError::DecryptionFailed(format!("Decryption failed: {:?}", e)))?;
            
        // Resize to the actual decrypted length
        in_out.truncate(decrypted_len);
        
        Ok(in_out)
    }
    
    /// Export a key (for secure storage)
    pub async fn export_key(&self, key_id: &str) -> Result<Vec<u8>, EncryptionError> {
        let keys = self.keys.read().map_err(|e| EncryptionError::Other(format!("Lock error: {}", e)))?;
        
        let key_data = keys.get(key_id).ok_or_else(|| EncryptionError::KeyNotFound(key_id.to_string()))?;
        
        // Serialize the key data
        let serialized = bincode::serialize(&(key_data.info.clone(), key_data.key_material.clone()))
            .map_err(|e| EncryptionError::SerializationError(format!("Failed to serialize key data: {}", e)))?;
            
        Ok(serialized)
    }
    
    /// Import a key (from secure storage)
    pub async fn import_key(&self, exported_key: &[u8]) -> Result<String, EncryptionError> {
        // Deserialize the key data
        let (info, key_material): (KeyInfo, Vec<u8>) = bincode::deserialize(exported_key)
            .map_err(|e| EncryptionError::SerializationError(format!("Failed to deserialize key data: {}", e)))?;
            
        let key_id = info.id.clone();
        
        // Store the key
        let key_data = KeyData {
            info,
            key_material,
        };
        
        let mut keys = self.keys.write().unwrap();
        keys.insert(key_id.clone(), key_data);
        
        Ok(key_id)
    }
    
    /// Generate a key ID from random bytes
    fn generate_key_id(id_bytes: &[u8]) -> Result<String, EncryptionError> {
        // Create a unique ID for the key (hex string of random bytes)
        let key_id = hex::encode(id_bytes);
        Ok(key_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_key_generation() {
        let service = StorageEncryptionService::new();
        let key_id = service.generate_key(vec!["federation1".to_string()]).await.unwrap();
        
        // Key ID should be a 32-character hex string
        assert_eq!(key_id.len(), 32);
        assert!(hex::decode(&key_id).is_ok());
    }
    
    #[tokio::test]
    async fn test_federation_key_access() {
        let service = StorageEncryptionService::new();
        
        // Generate a key for federation1
        let key_id = service.generate_key(vec!["federation1".to_string()]).await.unwrap();
        
        // federation1 should have access
        let has_access = service.federation_has_key_access("federation1", &key_id).await.unwrap();
        assert!(has_access);
        
        // federation2 should not have access
        let has_access = service.federation_has_key_access("federation2", &key_id).await;
        assert!(has_access.is_err());
        
        // Grant access to federation2
        service.grant_federation_key_access("federation2", &key_id).await.unwrap();
        
        // Now federation2 should have access
        let has_access = service.federation_has_key_access("federation2", &key_id).await.unwrap();
        assert!(has_access);
        
        // Revoke access from federation2
        service.revoke_federation_key_access("federation2", &key_id).await.unwrap();
        
        // federation2 should no longer have access
        let has_access = service.federation_has_key_access("federation2", &key_id).await;
        assert!(has_access.is_err());
    }
    
    #[tokio::test]
    async fn test_encrypt_decrypt() {
        let service = StorageEncryptionService::new();
        
        // Generate a key
        let key_id = service.generate_key(vec!["federation1".to_string()]).await.unwrap();
        
        // Test data to encrypt
        let plaintext = b"This is a test message.";
        
        // Encrypt the data
        let (ciphertext, metadata) = service.encrypt(plaintext, &key_id).await.unwrap();
        
        // The ciphertext should be different from the plaintext
        assert_ne!(&ciphertext, plaintext);
        
        // Decrypt the data
        let decrypted = service.decrypt(&ciphertext, &metadata).await.unwrap();
        
        // The decrypted data should match the original plaintext
        assert_eq!(&decrypted, plaintext);
    }
    
    #[tokio::test]
    async fn test_export_import_key() {
        let service = StorageEncryptionService::new();
        
        // Generate a key
        let original_key_id = service.generate_key(vec!["federation1".to_string()]).await.unwrap();
        
        // Export the key
        let exported_key = service.export_key(&original_key_id).await.unwrap();
        
        // Create a new service
        let new_service = StorageEncryptionService::new();
        
        // Import the key into the new service
        let imported_key_id = new_service.import_key(&exported_key).await.unwrap();
        
        // The key IDs should match
        assert_eq!(original_key_id, imported_key_id);
        
        // Test that the imported key works for encryption/decryption
        let plaintext = b"This is a test message.";
        
        // Encrypt with the new service
        let (ciphertext, metadata) = new_service.encrypt(plaintext, &imported_key_id).await.unwrap();
        
        // Decrypt with the new service
        let decrypted = new_service.decrypt(&ciphertext, &metadata).await.unwrap();
        
        // The decrypted data should match the original plaintext
        assert_eq!(&decrypted, plaintext);
    }
} 