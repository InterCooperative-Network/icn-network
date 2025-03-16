use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use ring::aead::{self, Aad, BoundKey, Nonce, NonceSequence, SealingKey, UnboundKey};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

// Maximum number of bytes that can be encrypted with a single key
const MAX_ENCRYPTIONS_PER_KEY: u32 = 1_000_000;

// Encryption error types
#[derive(Debug)]
pub enum EncryptionError {
    KeyGeneration(String),
    Encryption(String),
    Decryption(String),
    InvalidInput(String),
    KeyNotFound(String),
    KeyRotation(String),
    NonceExhausted(String),
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyGeneration(msg) => write!(f, "Key generation error: {}", msg),
            Self::Encryption(msg) => write!(f, "Encryption error: {}", msg),
            Self::Decryption(msg) => write!(f, "Decryption error: {}", msg),
            Self::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            Self::KeyNotFound(msg) => write!(f, "Key not found: {}", msg),
            Self::KeyRotation(msg) => write!(f, "Key rotation error: {}", msg),
            Self::NonceExhausted(msg) => write!(f, "Nonce exhausted: {}", msg),
        }
    }
}

impl Error for EncryptionError {}

// Nonce sequence for AES-GCM
struct AesGcmNonceSequence {
    nonce: [u8; 12],
    counter: u32,
}

impl AesGcmNonceSequence {
    fn new(nonce: [u8; 12]) -> Self {
        Self {
            nonce,
            counter: 0,
        }
    }
}

impl NonceSequence for AesGcmNonceSequence {
    fn advance(&mut self) -> Result<Nonce, ring::error::Unspecified> {
        if self.counter >= MAX_ENCRYPTIONS_PER_KEY {
            return Err(ring::error::Unspecified);
        }
        
        // Combine the nonce prefix with the counter
        // We use the first 8 bytes as a fixed prefix and the last 4 bytes as a counter
        let mut nonce_bytes = self.nonce;
        nonce_bytes[8..12].copy_from_slice(&self.counter.to_be_bytes());
        
        // Increment the counter
        self.counter += 1;
        
        Nonce::try_assume_unique_for_key(&nonce_bytes)
    }
}

// Encryption metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    pub algorithm: String,
    pub key_id: String,
    pub nonce_prefix: Vec<u8>,
    pub counter: u32,
    pub created_at: u64,
    pub additional_data: HashMap<String, String>,
}

// Key management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfo {
    pub key_id: String,
    pub algorithm: String,
    pub created_at: u64,
    pub use_count: u32,
    pub federation_ids: Vec<String>,
    // We don't serialize the actual key material
    #[serde(skip)]
    pub key_material: Vec<u8>,
}

// Storage encryption service
pub struct StorageEncryptionService {
    // Key management
    keys: RwLock<HashMap<String, KeyInfo>>,
    // Random number generator
    rng: SystemRandom,
    // Current default key ID
    current_key_id: RwLock<String>,
}

impl StorageEncryptionService {
    // Create a new storage encryption service
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
            rng: SystemRandom::new(),
            current_key_id: RwLock::new(String::new()),
        }
    }
    
    // Generate a new encryption key
    pub async fn generate_key(&self, federation_ids: Vec<String>) -> Result<String, EncryptionError> {
        let mut key_material = [0u8; 32]; // 256-bit key
        self.rng.fill(&mut key_material)
            .map_err(|_| EncryptionError::KeyGeneration("Failed to generate random key".to_string()))?;
        
        let key_id = self.generate_key_id()?;
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| EncryptionError::KeyGeneration(format!("Clock error: {}", e)))?
            .as_secs();
        
        let key_info = KeyInfo {
            key_id: key_id.clone(),
            algorithm: "AES-256-GCM".to_string(),
            created_at: now,
            use_count: 0,
            federation_ids,
            key_material: key_material.to_vec(),
        };
        
        // Store the key
        let mut keys = self.keys.write().await;
        keys.insert(key_id.clone(), key_info);
        
        // Set as current key if this is the first key
        if keys.len() == 1 {
            let mut current_key_id = self.current_key_id.write().await;
            *current_key_id = key_id.clone();
        }
        
        Ok(key_id)
    }
    
    // Set the current default encryption key
    pub async fn set_current_key(&self, key_id: &str) -> Result<(), EncryptionError> {
        let keys = self.keys.read().await;
        if !keys.contains_key(key_id) {
            return Err(EncryptionError::KeyNotFound(format!("Key not found: {}", key_id)));
        }
        
        let mut current_key_id = self.current_key_id.write().await;
        *current_key_id = key_id.to_string();
        
        Ok(())
    }
    
    // Encrypt data with a specific key or the current key
    pub async fn encrypt(
        &self,
        data: &[u8],
        key_id: Option<&str>,
        associated_data: Option<&[u8]>,
    ) -> Result<(Vec<u8>, EncryptionMetadata), EncryptionError> {
        // Get the key ID to use
        let key_id = match key_id {
            Some(id) => id.to_string(),
            None => {
                let current_key_id = self.current_key_id.read().await;
                if current_key_id.is_empty() {
                    return Err(EncryptionError::KeyNotFound("No current key set".to_string()));
                }
                current_key_id.clone()
            }
        };
        
        // Get the key info
        let mut keys = self.keys.write().await;
        let key_info = keys.get_mut(&key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(format!("Key not found: {}", key_id)))?;
        
        if key_info.use_count >= MAX_ENCRYPTIONS_PER_KEY {
            return Err(EncryptionError::NonceExhausted(
                format!("Key {} has reached its maximum usage", key_id)
            ));
        }
        
        // Create a nonce
        let mut nonce_prefix = [0u8; 12];
        self.rng.fill(&mut nonce_prefix)
            .map_err(|_| EncryptionError::Encryption("Failed to generate nonce".to_string()))?;
        
        // Set up the encryption key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_info.key_material)
            .map_err(|_| EncryptionError::Encryption("Failed to create encryption key".to_string()))?;
        
        let counter = key_info.use_count;
        let mut nonce_sequence = AesGcmNonceSequence::new(nonce_prefix);
        let mut sealing_key = SealingKey::new(unbound_key, nonce_sequence);
        
        // Encrypt the data
        let aad_bytes = associated_data.unwrap_or(&[]);
        let aad = Aad::from(aad_bytes);
        
        let mut in_out = data.to_vec();
        sealing_key.seal_in_place_append_tag(aad, &mut in_out)
            .map_err(|_| EncryptionError::Encryption("Failed to encrypt data".to_string()))?;
        
        // Update key usage
        key_info.use_count += 1;
        
        // Create encryption metadata
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| EncryptionError::Encryption(format!("Clock error: {}", e)))?
            .as_secs();
        
        let metadata = EncryptionMetadata {
            algorithm: key_info.algorithm.clone(),
            key_id: key_id.clone(),
            nonce_prefix: nonce_prefix.to_vec(),
            counter,
            created_at: now,
            additional_data: HashMap::new(),
        };
        
        Ok((in_out, metadata))
    }
    
    // Decrypt data with metadata
    pub async fn decrypt(
        &self,
        encrypted_data: &[u8],
        metadata: &EncryptionMetadata,
        associated_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, EncryptionError> {
        // Get the key info
        let keys = self.keys.read().await;
        let key_info = keys.get(&metadata.key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(format!("Key not found: {}", metadata.key_id)))?;
        
        // Create the nonce
        let mut nonce_bytes = [0u8; 12];
        if metadata.nonce_prefix.len() != 8 {
            return Err(EncryptionError::InvalidInput("Invalid nonce prefix length".to_string()));
        }
        nonce_bytes[0..8].copy_from_slice(&metadata.nonce_prefix[0..8]);
        nonce_bytes[8..12].copy_from_slice(&metadata.counter.to_be_bytes());
        
        let nonce = Nonce::try_assume_unique_for_key(&nonce_bytes)
            .map_err(|_| EncryptionError::Decryption("Invalid nonce".to_string()))?;
        
        // Set up the decryption key
        let unbound_key = UnboundKey::new(&aead::AES_256_GCM, &key_info.key_material)
            .map_err(|_| EncryptionError::Decryption("Failed to create decryption key".to_string()))?;
        
        // Decrypt the data
        let aad_bytes = associated_data.unwrap_or(&[]);
        let aad = Aad::from(aad_bytes);
        
        let mut in_out = encrypted_data.to_vec();
        let decrypted = ring::aead::open_in_place(unbound_key, nonce, aad, 0, &mut in_out)
            .map_err(|_| EncryptionError::Decryption("Failed to decrypt data".to_string()))?;
        
        Ok(decrypted.to_vec())
    }
    
    // Check if a federation has access to a key
    pub async fn federation_has_key_access(&self, federation_id: &str, key_id: &str) -> bool {
        let keys = self.keys.read().await;
        if let Some(key_info) = keys.get(key_id) {
            key_info.federation_ids.contains(&federation_id.to_string())
        } else {
            false
        }
    }
    
    // Grant a federation access to a key
    pub async fn grant_federation_key_access(&self, federation_id: &str, key_id: &str) -> Result<(), EncryptionError> {
        let mut keys = self.keys.write().await;
        let key_info = keys.get_mut(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(format!("Key not found: {}", key_id)))?;
        
        if !key_info.federation_ids.contains(&federation_id.to_string()) {
            key_info.federation_ids.push(federation_id.to_string());
        }
        
        Ok(())
    }
    
    // Revoke a federation's access to a key
    pub async fn revoke_federation_key_access(&self, federation_id: &str, key_id: &str) -> Result<(), EncryptionError> {
        let mut keys = self.keys.write().await;
        let key_info = keys.get_mut(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(format!("Key not found: {}", key_id)))?;
        
        key_info.federation_ids.retain(|id| id != federation_id);
        
        Ok(())
    }
    
    // Helper function to generate a key ID
    fn generate_key_id(&self) -> Result<String, EncryptionError> {
        let mut bytes = [0u8; 16];
        self.rng.fill(&mut bytes)
            .map_err(|_| EncryptionError::KeyGeneration("Failed to generate key ID".to_string()))?;
        
        Ok(format!("key-{}", hex::encode(bytes)))
    }
    
    // Export key info (without key material) for serialization
    pub async fn export_key_info(&self, key_id: &str) -> Result<KeyInfoExport, EncryptionError> {
        let keys = self.keys.read().await;
        let key_info = keys.get(key_id)
            .ok_or_else(|| EncryptionError::KeyNotFound(format!("Key not found: {}", key_id)))?;
        
        Ok(KeyInfoExport {
            key_id: key_info.key_id.clone(),
            algorithm: key_info.algorithm.clone(),
            created_at: key_info.created_at,
            use_count: key_info.use_count,
            federation_ids: key_info.federation_ids.clone(),
        })
    }
}

// Key info export structure (without key material)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyInfoExport {
    pub key_id: String,
    pub algorithm: String,
    pub created_at: u64,
    pub use_count: u32,
    pub federation_ids: Vec<String>,
} 