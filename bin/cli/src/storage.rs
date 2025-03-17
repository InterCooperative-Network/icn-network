//! Storage service module for the ICN CLI
//!
//! This module provides a secure, versioned, distributed storage system 
//! that supports multiple federations and end-to-end encryption.
//!
//! ## Features
//!
//! - **End-to-end encryption** using ChaCha20Poly1305
//! - **Data versioning** with automatic version tracking
//! - **Multi-federation support** for isolated storage areas
//! - **Cryptographic verification** of file integrity
//!
//! ## Usage
//!
//! The storage system can be used through the ICN CLI with commands like:
//!
//! ```
//! icn-cli storage init --path ./data
//! icn-cli storage generate-key
//! icn-cli storage put --file document.pdf --encrypted
//! icn-cli storage get --key document.pdf
//! icn-cli storage list
//! icn-cli storage history --key document.pdf
//! ```
//!
//! ## Implementation Details
//!
//! The storage system is built on top of the core ICN storage system
//! and extends it with additional functionality for versioning and encryption.
//! All metadata is stored separately from file contents to allow for
//! efficient retrieval of file information without loading large files.

use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, KeyInit},
    ChaCha20Poly1305, Nonce,
};
use icn_storage_system::{Storage, StorageExt, StorageOptions, create_storage};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{debug, info, warn};

// Add new imports for enhanced crypto
use aes_gcm::{
    aead::{Payload, Aead as AesAead, KeyInit as AesKeyInit},
    Aes256Gcm, Nonce as AesNonce,
};
use argon2::{
    password_hash::SaltString, Argon2, PasswordHasher
};
use rand::rngs::OsRng as RandOsRng;
use sha2::{Sha256, Digest};
use x25519_dalek::{PublicKey, StaticSecret};

/// Encryption format type
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EncryptionType {
    /// ChaCha20-Poly1305 symmetric encryption
    ChaCha20Poly1305,
    /// AES-256-GCM symmetric encryption
    Aes256Gcm,
    /// X25519 public key encryption (with ephemeral keys)
    X25519,
}

/// Cryptographic key types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CryptoKey {
    /// Symmetric encryption key (raw bytes)
    Symmetric(Vec<u8>),
    /// Asymmetric encryption public key
    Public(Vec<u8>),
    /// Asymmetric encryption private key
    Private(Vec<u8>),
    /// Password-derived key
    Password(String),
}

/// Metadata for encrypted content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    /// Type of encryption used
    pub encryption_type: EncryptionType,
    /// Nonce or IV used for encryption
    pub nonce: Vec<u8>,
    /// Key identifier (for key rotation/management)
    pub key_id: Option<String>,
    /// For asymmetric encryption, the ephemeral public key
    pub ephemeral_public_key: Option<Vec<u8>>,
    /// Optional authenticated data that was included
    pub authenticated_data: Option<Vec<u8>>,
}

/// Federation storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationConfig {
    /// Federation name
    pub name: String,
    /// Whether encryption is enabled for this federation
    pub encrypted: bool,
    /// Storage path for this federation
    pub path: PathBuf,
}

/// Versioned file metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionedFileMetadata {
    /// Original file name
    pub filename: String,
    /// Current version
    pub current_version: String,
    /// Creation timestamp
    pub created_at: u64,
    /// Last modified timestamp
    pub modified_at: u64,
    /// List of all versions
    pub versions: Vec<FileVersion>,
    /// Whether the file is encrypted
    pub encrypted: bool,
    /// Encryption metadata if encrypted
    pub encryption_meta: Option<EncryptionMetadata>,
}

/// Single file version information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileVersion {
    /// Version identifier (UUID)
    pub id: String,
    /// Version timestamp
    pub timestamp: u64,
    /// Version content hash
    pub content_hash: String,
    /// Version size in bytes
    pub size: usize,
}

/// Crypto service for managing encryption/decryption operations
pub struct CryptoService {
    /// Key store path for persistent keys
    key_store_path: PathBuf,
    /// In-memory key cache
    key_cache: std::collections::HashMap<String, CryptoKey>,
}

impl CryptoService {
    /// Create a new crypto service
    pub async fn new(key_store_path: impl AsRef<Path>) -> Result<Self> {
        let key_store_path = key_store_path.as_ref().to_path_buf();
        
        // Create key store directory if it doesn't exist
        if !key_store_path.exists() {
            fs::create_dir_all(&key_store_path).await?;
        }
        
        Ok(Self {
            key_store_path,
            key_cache: std::collections::HashMap::new(),
        })
    }
    
    /// Generate a new symmetric encryption key
    pub async fn generate_symmetric_key(&mut self, key_id: &str) -> Result<Vec<u8>> {
        let mut key = vec![0u8; 32]; // 256 bits
        OsRng.fill_bytes(&mut key);
        
        // Store key in cache and on disk
        self.key_cache.insert(key_id.to_string(), CryptoKey::Symmetric(key.clone()));
        self.store_key(key_id, &CryptoKey::Symmetric(key.clone())).await?;
        
        Ok(key)
    }
    
    /// Generate a new asymmetric key pair
    pub async fn generate_key_pair(&mut self, key_id: &str) -> Result<(Vec<u8>, Vec<u8>)> {
        // Generate X25519 key pair
        let private_key = StaticSecret::new(OsRng);
        let public_key = PublicKey::from(&private_key);
        
        let private_bytes = private_key.to_bytes().to_vec();
        let public_bytes = public_key.as_bytes().to_vec();
        
        // Store private key securely
        self.key_cache.insert(format!("{}_private", key_id), CryptoKey::Private(private_bytes.clone()));
        self.store_key(&format!("{}_private", key_id), &CryptoKey::Private(private_bytes.clone())).await?;
        
        // Store public key
        self.key_cache.insert(format!("{}_public", key_id), CryptoKey::Public(public_bytes.clone()));
        self.store_key(&format!("{}_public", key_id), &CryptoKey::Public(public_bytes.clone())).await?;
        
        Ok((public_bytes, private_bytes))
    }
    
    /// Derive a key from a password
    pub async fn derive_key_from_password(&self, password: &str, salt: Option<&[u8]>) -> Result<Vec<u8>> {
        // Generate salt if not provided
        let salt = if let Some(s) = salt {
            SaltString::from_b64(base64::encode(s).as_str())
                .map_err(|e| anyhow!("Failed to create salt: {}", e))?
        } else {
            SaltString::generate(&mut RandOsRng)
        };
        
        // Use Argon2 to derive a secure key
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow!("Failed to hash password: {}", e))?;
        
        // Convert hash to key material
        let key_material = password_hash.hash.unwrap().as_bytes();
        
        // Ensure key is right size for AES-256 (32 bytes)
        let mut hasher = Sha256::new();
        hasher.update(key_material);
        let key = hasher.finalize().to_vec();
        
        Ok(key)
    }
    
    /// Encrypt data with symmetric encryption
    pub async fn encrypt_symmetric(
        &self, 
        data: &[u8], 
        key: &[u8], 
        encryption_type: EncryptionType,
        authenticated_data: Option<&[u8]>,
    ) -> Result<(Vec<u8>, EncryptionMetadata)> {
        match encryption_type {
            EncryptionType::ChaCha20Poly1305 => {
                // Generate nonce
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = Nonce::from_slice(&nonce_bytes);
                
                // Create cipher
                let cipher = ChaCha20Poly1305::new_from_slice(key)
                    .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
                
                // Encrypt
                let ciphertext = if let Some(aad) = authenticated_data {
                    cipher.encrypt(nonce, aead::Payload { msg: data, aad })
                        .map_err(|e| anyhow!("Encryption failed: {}", e))?
                } else {
                    cipher.encrypt(nonce, data)
                        .map_err(|e| anyhow!("Encryption failed: {}", e))?
                };
                
                // Create metadata
                let metadata = EncryptionMetadata {
                    encryption_type,
                    nonce: nonce_bytes.to_vec(),
                    key_id: None,
                    ephemeral_public_key: None,
                    authenticated_data: authenticated_data.map(|d| d.to_vec()),
                };
                
                Ok((ciphertext, metadata))
            },
            EncryptionType::Aes256Gcm => {
                // Generate nonce
                let mut nonce_bytes = [0u8; 12];
                OsRng.fill_bytes(&mut nonce_bytes);
                let nonce = AesNonce::from_slice(&nonce_bytes);
                
                // Create cipher
                let cipher = Aes256Gcm::new_from_slice(key)
                    .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
                
                // Encrypt
                let ciphertext = if let Some(aad) = authenticated_data {
                    let payload = Payload { msg: data, aad };
                    cipher.encrypt(nonce, payload)
                        .map_err(|e| anyhow!("Encryption failed: {}", e))?
                } else {
                    cipher.encrypt(nonce, data)
                        .map_err(|e| anyhow!("Encryption failed: {}", e))?
                };
                
                // Create metadata
                let metadata = EncryptionMetadata {
                    encryption_type,
                    nonce: nonce_bytes.to_vec(),
                    key_id: None,
                    ephemeral_public_key: None,
                    authenticated_data: authenticated_data.map(|d| d.to_vec()),
                };
                
                Ok((ciphertext, metadata))
            },
            EncryptionType::X25519 => {
                Err(anyhow!("X25519 requires asymmetric encryption API"))
            }
        }
    }
    
    /// Decrypt data with symmetric encryption
    pub async fn decrypt_symmetric(
        &self,
        ciphertext: &[u8],
        key: &[u8],
        metadata: &EncryptionMetadata,
    ) -> Result<Vec<u8>> {
        match metadata.encryption_type {
            EncryptionType::ChaCha20Poly1305 => {
                // Create nonce
                let nonce = Nonce::from_slice(&metadata.nonce);
                
                // Create cipher
                let cipher = ChaCha20Poly1305::new_from_slice(key)
                    .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
                
                // Decrypt
                let plaintext = if let Some(aad) = &metadata.authenticated_data {
                    cipher.decrypt(nonce, aead::Payload { msg: ciphertext, aad: aad.as_slice() })
                        .map_err(|e| anyhow!("Decryption failed: {}", e))?
                } else {
                    cipher.decrypt(nonce, ciphertext)
                        .map_err(|e| anyhow!("Decryption failed: {}", e))?
                };
                
                Ok(plaintext)
            },
            EncryptionType::Aes256Gcm => {
                // Create nonce
                let nonce = AesNonce::from_slice(&metadata.nonce);
                
                // Create cipher
                let cipher = Aes256Gcm::new_from_slice(key)
                    .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
                
                // Decrypt
                let plaintext = if let Some(aad) = &metadata.authenticated_data {
                    let payload = Payload { msg: ciphertext, aad: aad.as_slice() };
                    cipher.decrypt(nonce, payload)
                        .map_err(|e| anyhow!("Decryption failed: {}", e))?
                } else {
                    cipher.decrypt(nonce, ciphertext)
                        .map_err(|e| anyhow!("Decryption failed: {}", e))?
                };
                
                Ok(plaintext)
            },
            EncryptionType::X25519 => {
                Err(anyhow!("X25519 requires asymmetric encryption API"))
            }
        }
    }
    
    /// Encrypt data with asymmetric encryption (for multiple recipients)
    pub async fn encrypt_asymmetric(
        &self,
        data: &[u8],
        recipient_public_keys: &[Vec<u8>],
        authenticated_data: Option<&[u8]>,
    ) -> Result<(Vec<u8>, EncryptionMetadata)> {
        // Generate ephemeral key pair
        let ephemeral_private = StaticSecret::new(OsRng);
        let ephemeral_public = PublicKey::from(&ephemeral_private);
        
        // Generate a random content encryption key
        let mut content_key = [0u8; 32];
        OsRng.fill_bytes(&mut content_key);
        
        // Create nonce for content encryption
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        
        // Encrypt the content with AES-GCM using the content key
        let nonce = AesNonce::from_slice(&nonce_bytes);
        let cipher = Aes256Gcm::new_from_slice(&content_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
        
        let ciphertext = if let Some(aad) = authenticated_data {
            let payload = Payload { msg: data, aad };
            cipher.encrypt(nonce, payload)
                .map_err(|e| anyhow!("Encryption failed: {}", e))?
        } else {
            cipher.encrypt(nonce, data)
                .map_err(|e| anyhow!("Encryption failed: {}", e))?
        };
        
        // Generate per-recipient encrypted content keys
        let mut recipient_keys = Vec::new();
        
        for recipient_key_bytes in recipient_public_keys {
            if recipient_key_bytes.len() != 32 {
                return Err(anyhow!("Invalid public key length"));
            }
            
            // Convert bytes to PublicKey
            let recipient_key_array: [u8; 32] = recipient_key_bytes.clone().try_into()
                .map_err(|_| anyhow!("Invalid public key format"))?;
            let recipient_public = PublicKey::from(recipient_key_array);
            
            // Generate shared secret
            let shared_secret = ephemeral_private.diffie_hellman(&recipient_public);
            
            // Use shared secret to wrap content key
            let mut key_wrapping_key = [0u8; 32];
            let salt = b"ICN-KEK"; // Key Encryption Key
            let info = b"";
            
            // HKDF to derive key wrapping key
            let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), shared_secret.as_bytes());
            hk.expand(info, &mut key_wrapping_key)
                .map_err(|_| anyhow!("HKDF expansion failed"))?;
            
            // Encrypt content key with key wrapping key
            let mut key_nonce_bytes = [0u8; 12];
            OsRng.fill_bytes(&mut key_nonce_bytes);
            let key_nonce = AesNonce::from_slice(&key_nonce_bytes);
            
            let key_cipher = Aes256Gcm::new_from_slice(&key_wrapping_key)
                .map_err(|e| anyhow!("Failed to create key cipher: {}", e))?;
            
            let encrypted_key = key_cipher.encrypt(key_nonce, &content_key[..])
                .map_err(|e| anyhow!("Key encryption failed: {}", e))?;
            
            // Save recipient info (recipient_id, encrypted_key, nonce)
            recipient_keys.push((
                base64::encode(recipient_key_bytes),
                encrypted_key,
                key_nonce_bytes.to_vec(),
            ));
        }
        
        // Serialize recipient keys into the format to be stored with the ciphertext
        let recipient_data = serde_json::to_vec(&recipient_keys)?;
        
        // Prepend the recipient data to the ciphertext
        let mut final_ciphertext = Vec::new();
        final_ciphertext.extend_from_slice(&(recipient_data.len() as u32).to_be_bytes());
        final_ciphertext.extend_from_slice(&recipient_data);
        final_ciphertext.extend_from_slice(&ciphertext);
        
        // Create metadata
        let metadata = EncryptionMetadata {
            encryption_type: EncryptionType::X25519,
            nonce: nonce_bytes.to_vec(),
            key_id: None,
            ephemeral_public_key: Some(ephemeral_public.as_bytes().to_vec()),
            authenticated_data: authenticated_data.map(|d| d.to_vec()),
        };
        
        Ok((final_ciphertext, metadata))
    }
    
    /// Decrypt data with asymmetric encryption
    pub async fn decrypt_asymmetric(
        &self,
        ciphertext: &[u8],
        private_key: &[u8],
        metadata: &EncryptionMetadata,
    ) -> Result<Vec<u8>> {
        if metadata.encryption_type != EncryptionType::X25519 {
            return Err(anyhow!("Incorrect encryption type"));
        }
        
        if private_key.len() != 32 {
            return Err(anyhow!("Invalid private key length"));
        }
        
        // Get ephemeral public key
        let ephemeral_public_bytes = metadata.ephemeral_public_key
            .as_ref()
            .ok_or_else(|| anyhow!("Missing ephemeral public key"))?;
        
        if ephemeral_public_bytes.len() != 32 {
            return Err(anyhow!("Invalid ephemeral public key length"));
        }
        
        // Convert bytes to keys
        let private_key_array: [u8; 32] = private_key.try_into()
            .map_err(|_| anyhow!("Invalid private key format"))?;
        let private = StaticSecret::from(private_key_array);
        
        let ephemeral_public_array: [u8; 32] = ephemeral_public_bytes.clone().try_into()
            .map_err(|_| anyhow!("Invalid ephemeral public key format"))?;
        let ephemeral_public = PublicKey::from(ephemeral_public_array);
        
        // Read recipient data length
        if ciphertext.len() < 4 {
            return Err(anyhow!("Ciphertext too short"));
        }
        
        let mut length_bytes = [0u8; 4];
        length_bytes.copy_from_slice(&ciphertext[0..4]);
        let recipient_data_len = u32::from_be_bytes(length_bytes) as usize;
        
        if ciphertext.len() < 4 + recipient_data_len {
            return Err(anyhow!("Ciphertext too short for recipient data"));
        }
        
        // Extract recipient data and actual ciphertext
        let recipient_data: Vec<(String, Vec<u8>, Vec<u8>)> = serde_json::from_slice(
            &ciphertext[4..4 + recipient_data_len]
        )?;
        
        let actual_ciphertext = &ciphertext[4 + recipient_data_len..];
        
        // Compute public key from private key to find matching recipient
        let our_public = PublicKey::from(&private);
        let our_public_b64 = base64::encode(our_public.as_bytes());
        
        // Find our encrypted key
        let mut our_key_info = None;
        for (recipient_id, encrypted_key, key_nonce) in recipient_data {
            if recipient_id == our_public_b64 {
                our_key_info = Some((encrypted_key, key_nonce));
                break;
            }
        }
        
        let (encrypted_key, key_nonce) = our_key_info
            .ok_or_else(|| anyhow!("No matching recipient found"))?;
        
        // Generate shared secret
        let shared_secret = private.diffie_hellman(&ephemeral_public);
        
        // Derive key wrapping key
        let mut key_wrapping_key = [0u8; 32];
        let salt = b"ICN-KEK"; // Key Encryption Key
        let info = b"";
        
        // HKDF to derive key wrapping key
        let hk = hkdf::Hkdf::<Sha256>::new(Some(salt), shared_secret.as_bytes());
        hk.expand(info, &mut key_wrapping_key)
            .map_err(|_| anyhow!("HKDF expansion failed"))?;
        
        // Decrypt the content key
        let key_nonce = AesNonce::from_slice(&key_nonce);
        let key_cipher = Aes256Gcm::new_from_slice(&key_wrapping_key)
            .map_err(|e| anyhow!("Failed to create key cipher: {}", e))?;
        
        let content_key = key_cipher.decrypt(key_nonce, encrypted_key.as_slice())
            .map_err(|e| anyhow!("Key decryption failed: {}", e))?;
        
        // Finally decrypt the content with the content key
        let nonce = AesNonce::from_slice(&metadata.nonce);
        let cipher = Aes256Gcm::new_from_slice(&content_key)
            .map_err(|e| anyhow!("Failed to create cipher: {}", e))?;
        
        let plaintext = if let Some(aad) = &metadata.authenticated_data {
            let payload = Payload { msg: actual_ciphertext, aad: aad.as_slice() };
            cipher.decrypt(nonce, payload)
                .map_err(|e| anyhow!("Decryption failed: {}", e))?
        } else {
            cipher.decrypt(nonce, actual_ciphertext)
                .map_err(|e| anyhow!("Decryption failed: {}", e))?
        };
        
        Ok(plaintext)
    }
    
    /// Store a key to disk
    async fn store_key(&self, key_id: &str, key: &CryptoKey) -> Result<()> {
        let key_path = self.key_store_path.join(format!("{}.key", key_id));
        
        // Serialize key
        let serialized = serde_json::to_vec(key)?;
        
        // Write to file with restrictive permissions
        // TODO: Use platform-specific file permissions API
        fs::write(&key_path, &serialized).await?;
        
        Ok(())
    }
    
    /// Load a key from disk
    async fn load_key(&mut self, key_id: &str) -> Result<CryptoKey> {
        // Check cache first
        if let Some(key) = self.key_cache.get(key_id) {
            return Ok(key.clone());
        }
        
        // Load from disk
        let key_path = self.key_store_path.join(format!("{}.key", key_id));
        let data = fs::read(&key_path).await?;
        let key: CryptoKey = serde_json::from_slice(&data)?;
        
        // Update cache
        self.key_cache.insert(key_id.to_string(), key.clone());
        
        Ok(key)
    }
    
    /// Import a key 
    pub async fn import_key(&mut self, key_id: &str, key: &CryptoKey) -> Result<()> {
        // Store in cache and on disk
        self.key_cache.insert(key_id.to_string(), key.clone());
        self.store_key(key_id, key).await?;
        
        Ok(())
    }
    
    /// Export a key
    pub async fn export_key(&self, key_id: &str) -> Result<CryptoKey> {
        // Check cache first
        if let Some(key) = self.key_cache.get(key_id) {
            return Ok(key.clone());
        }
        
        // Load from disk
        let key_path = self.key_store_path.join(format!("{}.key", key_id));
        let data = fs::read(&key_path).await?;
        let key: CryptoKey = serde_json::from_slice(&data)?;
        
        Ok(key)
    }
}

/// Storage service for managing files with encryption and versioning
pub struct StorageService {
    /// Base storage path
    base_path: PathBuf,
    /// Federation configurations
    federations: Vec<FederationConfig>,
    /// Storage instances per federation
    storages: std::collections::HashMap<String, std::sync::Arc<dyn Storage>>,
    /// Crypto service for encryption/decryption
    crypto_service: Option<CryptoService>,
}

impl StorageService {
    /// Create a new storage service
    pub async fn new(base_path: impl AsRef<Path>) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        
        // Create base directory if it doesn't exist
        if !base_path.exists() {
            fs::create_dir_all(&base_path).await?;
        }
        
        // Read federation configs if they exist
        let config_path = base_path.join("federations.json");
        let federations = if config_path.exists() {
            let data = fs::read(&config_path).await?;
            serde_json::from_slice(&data)?
        } else {
            // Create default federation
            let default_federation = FederationConfig {
                name: "default".to_string(),
                encrypted: false,
                path: base_path.join("default"),
            };
            
            fs::create_dir_all(&default_federation.path).await?;
            
            let federations = vec![default_federation];
            let data = serde_json::to_vec(&federations)?;
            fs::write(&config_path, &data).await?;
            
            federations
        };
        
        // Initialize storage for each federation
        let mut storages = std::collections::HashMap::new();
        for federation in &federations {
            let options = StorageOptions {
                base_dir: federation.path.clone(),
                sync_writes: true,
                compress: false,
            };
            
            let storage = create_storage(options).await?;
            storages.insert(federation.name.clone(), storage);
        }
        
        // Initialize CryptoService
        let crypto_service = CryptoService::new(base_path.join("keys")).await?;
        
        Ok(Self {
            base_path,
            federations,
            storages,
            crypto_service: Some(crypto_service),
        })
    }
    
    /// Initialize a new federation
    pub async fn init_federation(&mut self, name: &str, encrypted: bool) -> Result<()> {
        // Check if federation already exists
        if self.federations.iter().any(|f| f.name == name) {
            return Err(anyhow!("Federation '{}' already exists", name));
        }
        
        // Create federation directory
        let fed_path = self.base_path.join(name);
        fs::create_dir_all(&fed_path).await?;
        
        // Create federation config
        let federation = FederationConfig {
            name: name.to_string(),
            encrypted,
            path: fed_path,
        };
        
        // Initialize storage
        let options = StorageOptions {
            base_dir: federation.path.clone(),
            sync_writes: true,
            compress: false,
        };
        
        let storage = create_storage(options).await?;
        self.storages.insert(name.to_string(), storage);
        
        // Add to federations list
        self.federations.push(federation);
        
        // Save updated federation configs
        let config_path = self.base_path.join("federations.json");
        let data = serde_json::to_vec(&self.federations)?;
        fs::write(&config_path, &data).await?;
        
        // If encryption is enabled, generate federation key
        if encrypted {
            if let Some(crypto_service) = &mut self.crypto_service {
                // Generate encryption key for the federation
                let _key = crypto_service.generate_symmetric_key(&format!("federation_{}", name)).await?;
                info!("Generated encryption key for federation '{}'", name);
            }
        }
        
        info!("Initialized federation '{}'", name);
        Ok(())
    }
    
    /// Store a file in the specified federation
    pub async fn store_file(
        &self, 
        file_path: impl AsRef<Path>, 
        key: &str, 
        federation: &str, 
        encrypted: bool
    ) -> Result<()> {
        // Check if federation exists
        let federation_cfg = self.get_federation(federation)?;
        
        // Read file content
        let file_path = file_path.as_ref();
        let file_content = fs::read(file_path).await?;
        
        // Generate version ID (UUID)
        let version_id = uuid::Uuid::new_v4().to_string();
        
        // Get current timestamp
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Get storage for the federation
        let storage = self.storages.get(federation).ok_or_else(|| anyhow!("Federation storage not found"))?;
        
        // Prepare content to store (either encrypted or plain)
        let (stored_content, encryption_meta) = if encrypted {
            // Use CryptoService for encryption
            if let Some(crypto_service) = &self.crypto_service {
                // Get or generate federation key
                let fed_key_id = format!("federation_{}", federation);
                let fed_key = match crypto_service.export_key(&fed_key_id).await {
                    Ok(CryptoKey::Symmetric(key_data)) => key_data,
                    _ => {
                        return Err(anyhow!(
                            "Could not find encryption key for federation {}", federation
                        ));
                    }
                };
                
                // Calculate content hash (of unencrypted content)
                let content_hash = format!("{:x}", sha2::Sha256::digest(&file_content));
                
                // Use authenticated data to verify integrity
                let auth_data = content_hash.as_bytes();
                
                // Encrypt with AES-GCM (preferred over ChaCha20Poly1305 for hardware acceleration)
                let (encrypted_data, meta) = crypto_service.encrypt_symmetric(
                    &file_content, 
                    &fed_key, 
                    EncryptionType::Aes256Gcm,
                    Some(auth_data)
                ).await?;
                
                // Return encrypted content and metadata
                (encrypted_data, Some(meta))
            } else {
                return Err(anyhow!("Encryption requested but crypto service not initialized"));
            }
        } else {
            // Store unencrypted
            (file_content.clone(), None)
        };
        
        // Calculate content hash
        let content_hash = format!("{:x}", sha2::Sha256::digest(&file_content));
        
        // Check if file already exists
        let metadata_key = format!("meta:{}", key);
        
        let metadata: Option<VersionedFileMetadata> = storage.get(&metadata_key).await?;
        
        let metadata = if let Some(mut existing_metadata) = metadata {
            // Update existing metadata with new version
            let new_version = FileVersion {
                id: version_id.clone(),
                timestamp,
                content_hash: content_hash.clone(),
                size: file_content.len(),
            };
            
            existing_metadata.versions.push(new_version);
            existing_metadata.current_version = version_id.clone();
            existing_metadata.modified_at = timestamp;
            existing_metadata.encrypted = encrypted;
            existing_metadata.encryption_meta = encryption_meta;
            
            existing_metadata
        } else {
            // Create new metadata
            let filename = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(key)
                .to_string();
            
            VersionedFileMetadata {
                filename,
                current_version: version_id.clone(),
                created_at: timestamp,
                modified_at: timestamp,
                versions: vec![FileVersion {
                    id: version_id.clone(),
                    timestamp,
                    content_hash: content_hash.clone(),
                    size: file_content.len(),
                }],
                encrypted,
                encryption_meta,
            }
        };
        
        // Store metadata
        storage.put(&metadata_key, &metadata).await?;
        
        // Store content with version ID
        let content_key = format!("content:{}:{}", key, version_id);
        storage.put_bytes(&content_key, &stored_content).await?;
        
        debug!("Stored file {} with key {} in federation {} (encrypted: {})", 
            file_path.display(), key, federation, encrypted);
        
        Ok(())
    }
    
    /// Retrieve a file from the specified federation
    pub async fn retrieve_file(
        &self,
        key: &str,
        output_path: impl AsRef<Path>,
        federation: &str,
        version: Option<&str>,
    ) -> Result<()> {
        // Check if federation exists
        self.get_federation(federation)?;
        
        // Get storage for the federation
        let storage = self.storages.get(federation).ok_or_else(|| anyhow!("Federation storage not found"))?;
        
        // Get metadata
        let metadata_key = format!("meta:{}", key);
        let metadata: VersionedFileMetadata = storage.get(&metadata_key).await?
            .ok_or_else(|| anyhow!("File not found: {}", key))?;
        
        // Determine which version to retrieve
        let version_id = version.unwrap_or(&metadata.current_version);
        
        // Check if version exists
        if !metadata.versions.iter().any(|v| v.id == *version_id) {
            return Err(anyhow!("Version {} not found for {}", version_id, key));
        }
        
        // Get content
        let content_key = format!("content:{}:{}", key, version_id);
        let encrypted_content = storage.get_bytes(&content_key).await?
            .ok_or_else(|| anyhow!("Content not found for version {} of {}", version_id, key))?;
        
        // Decrypt if necessary
        let content = if metadata.encrypted {
            if let Some(crypto_service) = &self.crypto_service {
                // Get encryption metadata
                let encryption_meta = metadata.encryption_meta
                    .as_ref()
                    .ok_or_else(|| anyhow!("File is marked as encrypted but missing encryption metadata"))?;
                
                // Get federation key
                let fed_key_id = format!("federation_{}", federation);
                let fed_key = match crypto_service.export_key(&fed_key_id).await {
                    Ok(CryptoKey::Symmetric(key_data)) => key_data,
                    _ => {
                        return Err(anyhow!(
                            "Could not find encryption key for federation {}", federation
                        ));
                    }
                };
                
                // Decrypt data
                crypto_service.decrypt_symmetric(&encrypted_content, &fed_key, encryption_meta).await?
            } else {
                return Err(anyhow!("File is encrypted but crypto service not initialized"));
            }
        } else {
            encrypted_content
        };
        
        // Write to output file
        let output_path = output_path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = output_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }
        
        fs::write(output_path, &content).await?;
        
        debug!("Retrieved file {} to {} (version: {})", 
            key, output_path.display(), version_id);
        
        Ok(())
    }
    
    /// List files in the specified federation
    pub async fn list_files(&self, federation: &str, prefix: Option<&str>) -> Result<Vec<VersionedFileMetadata>> {
        // Check if federation exists
        self.get_federation(federation)?;
        
        // Get storage for the federation
        let storage = self.storages.get(federation).ok_or_else(|| anyhow!("Federation storage not found"))?;
        
        // List all metadata keys
        let meta_prefix = prefix.map_or("meta:".to_string(), |p| format!("meta:{}", p));
        let meta_keys = storage.list_keys(&meta_prefix).await?;
        
        // Get metadata for each key
        let mut files = Vec::new();
        for meta_key in meta_keys {
            if let Some(metadata) = storage.get::<VersionedFileMetadata>(&meta_key).await? {
                files.push(metadata);
            }
        }
        
        // Sort by last modified (newest first)
        files.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        
        Ok(files)
    }
    
    /// Get version history for a file
    pub async fn get_history(&self, key: &str, federation: &str, limit: usize) -> Result<Vec<FileVersion>> {
        // Check if federation exists
        self.get_federation(federation)?;
        
        // Get storage for the federation
        let storage = self.storages.get(federation).ok_or_else(|| anyhow!("Federation storage not found"))?;
        
        // Get metadata
        let metadata_key = format!("meta:{}", key);
        let metadata: Option<VersionedFileMetadata> = storage.get(&metadata_key).await?;
        
        if let Some(metadata) = metadata {
            // Return versions (newest first, limited to the specified number)
            let mut versions = metadata.versions;
            versions.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            Ok(versions.into_iter().take(limit).collect())
        } else {
            Err(anyhow!("File not found: {}", key))
        }
    }
    
    /// Generate encryption key for a federation
    pub async fn generate_key(&self, output_path: impl AsRef<Path>) -> Result<()> {
        let output_path = output_path.as_ref().to_path_buf();
        
        if let Some(crypto_service) = &self.crypto_service {
            // Generate a new symmetric key
            let federation_id = "default";
            let key_id = format!("federation_{}", federation_id);
            let key = crypto_service.generate_symmetric_key(&key_id).await?;
            
            // Write key to the specified output file
            fs::write(&output_path, &key).await?;
            
            info!("Generated new encryption key for federation {}", federation_id);
            info!("Key stored in {}", output_path.display());
            
            Ok(())
        } else {
            Err(anyhow!("Crypto service not initialized"))
        }
    }
    
    /// Export an encryption key for sharing
    pub async fn export_encryption_key(&self, federation: &str, output_path: impl AsRef<Path>) -> Result<()> {
        let output_path = output_path.as_ref().to_path_buf();
        
        // Check if federation exists
        self.get_federation(federation)?;
        
        if let Some(crypto_service) = &self.crypto_service {
            // Get the federation key
            let key_id = format!("federation_{}", federation);
            let key = crypto_service.export_key(&key_id).await?;
            
            // Serialize key data
            let key_data = serde_json::to_vec(&key)?;
            
            // Write to output file
            fs::write(&output_path, &key_data).await?;
            
            info!("Exported encryption key for federation {} to {}", 
                federation, output_path.display());
            
            Ok(())
        } else {
            Err(anyhow!("Crypto service not initialized"))
        }
    }
    
    /// Import an encryption key
    pub async fn import_encryption_key(&self, federation: &str, key_path: impl AsRef<Path>) -> Result<()> {
        let key_path = key_path.as_ref().to_path_buf();
        
        // Check if federation exists
        self.get_federation(federation)?;
        
        if let Some(crypto_service) = &self.crypto_service {
            // Read key data
            let key_data = fs::read(&key_path).await?;
            
            // Parse key
            let key: CryptoKey = serde_json::from_slice(&key_data)?;
            
            // Import key
            let key_id = format!("federation_{}", federation);
            crypto_service.import_key(&key_id, &key).await?;
            
            info!("Imported encryption key for federation {} from {}", 
                federation, key_path.display());
            
            Ok(())
        } else {
            Err(anyhow!("Crypto service not initialized"))
        }
    }
    
    /// Generate a key pair for asymmetric encryption
    pub async fn generate_key_pair(&self, output_dir: impl AsRef<Path>) -> Result<()> {
        let output_dir = output_dir.as_ref().to_path_buf();
        
        // Create output directory if it doesn't exist
        if !output_dir.exists() {
            fs::create_dir_all(&output_dir).await?;
        }
        
        if let Some(crypto_service) = &self.crypto_service {
            // Generate a new key pair
            let key_id = format!("user_{}", uuid::Uuid::new_v4());
            let (public_key, private_key) = crypto_service.generate_key_pair(&key_id).await?;
            
            // Write keys to files
            fs::write(output_dir.join("public.key"), &public_key).await?;
            fs::write(output_dir.join("private.key"), &private_key).await?;
            
            // Write key ID to a file
            fs::write(output_dir.join("key_id.txt"), key_id.as_bytes()).await?;
            
            info!("Generated new asymmetric key pair");
            info!("Public key stored in {}", output_dir.join("public.key").display());
            info!("Private key stored in {}", output_dir.join("private.key").display());
            
            Ok(())
        } else {
            Err(anyhow!("Crypto service not initialized"))
        }
    }
    
    /// Encrypt a file for specific recipients
    pub async fn encrypt_for_recipients(
        &self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        recipient_keys: &[Vec<u8>],
    ) -> Result<()> {
        let input_path = input_path.as_ref().to_path_buf();
        let output_path = output_path.as_ref().to_path_buf();
        
        if let Some(crypto_service) = &self.crypto_service {
            // Read input file
            let file_content = fs::read(&input_path).await?;
            
            // Calculate content hash for authentication
            let content_hash = format!("{:x}", sha2::Sha256::digest(&file_content));
            let auth_data = content_hash.as_bytes();
            
            // Encrypt for recipients
            let (encrypted_data, metadata) = crypto_service.encrypt_asymmetric(
                &file_content, recipient_keys, Some(auth_data)
            ).await?;
            
            // Serialize metadata
            let metadata_json = serde_json::to_vec(&metadata)?;
            
            // Write encrypted file with metadata header
            let mut output_file = tokio::fs::File::create(&output_path).await?;
            
            // Write metadata length as u32 (4 bytes)
            output_file.write_all(&(metadata_json.len() as u32).to_be_bytes()).await?;
            
            // Write metadata
            output_file.write_all(&metadata_json).await?;
            
            // Write encrypted content
            output_file.write_all(&encrypted_data).await?;
            
            // Flush
            output_file.flush().await?;
            
            info!("Encrypted file {} for {} recipients and stored in {}", 
                input_path.display(), recipient_keys.len(), output_path.display());
            
            Ok(())
        } else {
            Err(anyhow!("Crypto service not initialized"))
        }
    }
    
    /// Decrypt a file encrypted for a specific recipient
    pub async fn decrypt_with_private_key(
        &self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
        private_key: &[u8],
    ) -> Result<()> {
        let input_path = input_path.as_ref().to_path_buf();
        let output_path = output_path.as_ref().to_path_buf();
        
        if let Some(crypto_service) = &self.crypto_service {
            // Open input file
            let mut input_file = tokio::fs::File::open(&input_path).await?;
            
            // Read metadata length (4 bytes)
            let mut length_bytes = [0u8; 4];
            input_file.read_exact(&mut length_bytes).await?;
            let metadata_len = u32::from_be_bytes(length_bytes) as usize;
            
            // Read metadata
            let mut metadata_bytes = vec![0u8; metadata_len];
            input_file.read_exact(&mut metadata_bytes).await?;
            let metadata: EncryptionMetadata = serde_json::from_slice(&metadata_bytes)?;
            
            // Read encrypted data
            let mut encrypted_data = Vec::new();
            input_file.read_to_end(&mut encrypted_data).await?;
            
            // Decrypt data
            let decrypted_data = crypto_service.decrypt_asymmetric(
                &encrypted_data, private_key, &metadata
            ).await?;
            
            // Write decrypted data to output file
            fs::write(&output_path, &decrypted_data).await?;
            
            info!("Decrypted file {} and stored in {}", 
                input_path.display(), output_path.display());
            
            Ok(())
        } else {
            Err(anyhow!("Crypto service not initialized"))
        }
    }
    
    /// Get a federation configuration
    fn get_federation(&self, name: &str) -> Result<&FederationConfig> {
        self.federations.iter()
            .find(|f| f.name == name)
            .ok_or_else(|| anyhow!("Federation not found: {}", name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[tokio::test]
    async fn test_crypto_service_symmetric() -> Result<()> {
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let crypto_service = CryptoService::new(temp_dir.path()).await?;
        
        // Test data
        let test_data = b"This is a test message for symmetric encryption";
        
        // Test ChaCha20Poly1305 encryption/decryption
        let key_id = "test_chacha_key";
        let mut crypto_service = crypto_service;
        let key = crypto_service.generate_symmetric_key(key_id).await?;
        
        let (ciphertext, metadata) = crypto_service.encrypt_symmetric(
            test_data, 
            &key, 
            EncryptionType::ChaCha20Poly1305,
            None
        ).await?;
        
        let plaintext = crypto_service.decrypt_symmetric(
            &ciphertext, 
            &key, 
            &metadata
        ).await?;
        
        assert_eq!(plaintext, test_data);
        
        // Test AES-GCM encryption/decryption
        let key_id = "test_aes_key";
        let key = crypto_service.generate_symmetric_key(key_id).await?;
        
        let (ciphertext, metadata) = crypto_service.encrypt_symmetric(
            test_data, 
            &key, 
            EncryptionType::Aes256Gcm,
            None
        ).await?;
        
        let plaintext = crypto_service.decrypt_symmetric(
            &ciphertext, 
            &key, 
            &metadata
        ).await?;
        
        assert_eq!(plaintext, test_data);
        
        // Test with authenticated data
        let auth_data = b"Additional authenticated data";
        
        let (ciphertext, metadata) = crypto_service.encrypt_symmetric(
            test_data, 
            &key, 
            EncryptionType::Aes256Gcm,
            Some(auth_data)
        ).await?;
        
        let plaintext = crypto_service.decrypt_symmetric(
            &ciphertext, 
            &key, 
            &metadata
        ).await?;
        
        assert_eq!(plaintext, test_data);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_crypto_service_asymmetric() -> Result<()> {
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let mut crypto_service = CryptoService::new(temp_dir.path()).await?;
        
        // Test data
        let test_data = b"This is a test message for asymmetric encryption";
        
        // Generate key pairs for multiple recipients
        let (pub_key1, priv_key1) = crypto_service.generate_key_pair("recipient1").await?;
        let (pub_key2, priv_key2) = crypto_service.generate_key_pair("recipient2").await?;
        
        // Encrypt for both recipients
        let recipient_keys = vec![pub_key1.clone(), pub_key2.clone()];
        let (ciphertext, metadata) = crypto_service.encrypt_asymmetric(
            test_data, 
            &recipient_keys, 
            None
        ).await?;
        
        // Recipient 1 decrypts
        let plaintext1 = crypto_service.decrypt_asymmetric(
            &ciphertext, 
            &priv_key1, 
            &metadata
        ).await?;
        
        // Recipient 2 decrypts
        let plaintext2 = crypto_service.decrypt_asymmetric(
            &ciphertext, 
            &priv_key2, 
            &metadata
        ).await?;
        
        // Both recipients should get the same plaintext
        assert_eq!(plaintext1, test_data);
        assert_eq!(plaintext2, test_data);
        
        // Test with authenticated data
        let auth_data = b"Additional authenticated data";
        
        let (ciphertext, metadata) = crypto_service.encrypt_asymmetric(
            test_data, 
            &recipient_keys, 
            Some(auth_data)
        ).await?;
        
        let plaintext = crypto_service.decrypt_asymmetric(
            &ciphertext, 
            &priv_key1, 
            &metadata
        ).await?;
        
        assert_eq!(plaintext, test_data);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_password_derived_keys() -> Result<()> {
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let crypto_service = CryptoService::new(temp_dir.path()).await?;
        
        // Test data
        let test_data = b"This is a test message for password-based encryption";
        
        // Generate key from password
        let password = "secure_password_123";
        let salt = b"static_salt_for_test";
        
        let key = crypto_service.derive_key_from_password(password, Some(salt)).await?;
        
        // Ensure key is the right length for AES-256
        assert_eq!(key.len(), 32);
        
        // Test encrypting with password-derived key
        let (ciphertext, metadata) = crypto_service.encrypt_symmetric(
            test_data, 
            &key, 
            EncryptionType::Aes256Gcm,
            None
        ).await?;
        
        // Generate the same key again with the same password and salt
        let key2 = crypto_service.derive_key_from_password(password, Some(salt)).await?;
        
        // Keys should be identical
        assert_eq!(key, key2);
        
        // Decrypt with the regenerated key
        let plaintext = crypto_service.decrypt_symmetric(
            &ciphertext, 
            &key2, 
            &metadata
        ).await?;
        
        assert_eq!(plaintext, test_data);
        
        Ok(())
    }
    
    #[tokio::test]
    async fn test_key_export_import() -> Result<()> {
        // Create a temporary directory for the test
        let temp_dir = tempdir()?;
        let mut crypto_service = CryptoService::new(temp_dir.path()).await?;
        
        // Generate a key
        let key_id = "export_test_key";
        let original_key = crypto_service.generate_symmetric_key(key_id).await?;
        
        // Export the key
        let exported_key = crypto_service.export_key(key_id).await?;
        
        // Create a new crypto service instance
        let temp_dir2 = tempdir()?;
        let mut crypto_service2 = CryptoService::new(temp_dir2.path()).await?;
        
        // Import the key
        crypto_service2.import_key(key_id, &exported_key).await?;
        
        // Export it again
        let exported_key2 = crypto_service2.export_key(key_id).await?;
        
        // Compare the exported keys
        match (exported_key, exported_key2) {
            (CryptoKey::Symmetric(k1), CryptoKey::Symmetric(k2)) => {
                assert_eq!(k1, k2);
                assert_eq!(k1, original_key);
            },
            _ => panic!("Exported keys are not the expected type"),
        }
        
        Ok(())
    }
} 