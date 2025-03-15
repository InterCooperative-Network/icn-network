use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

/// Storage-related errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Not a directory: {0}")]
    NotADirectory(String),
    
    #[error("Unexpected error: {0}")]
    Other(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage options
#[derive(Debug, Clone)]
pub struct StorageOptions {
    pub sync_write: bool,
    pub create_dirs: bool,
    pub use_cache: bool,
}

impl Default for StorageOptions {
    fn default() -> Self {
        StorageOptions {
            sync_write: true,
            create_dirs: true,
            use_cache: true,
        }
    }
}

/// The core Storage trait defining the operations all storage implementations must support
#[async_trait]
pub trait Storage: Send + Sync + 'static {
    /// Store data at the specified key
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()>;
    
    /// Retrieve data from the specified key
    async fn get(&self, key: &str) -> StorageResult<Vec<u8>>;
    
    /// Delete data at the specified key
    async fn delete(&self, key: &str) -> StorageResult<()>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> StorageResult<bool>;
    
    /// List all keys with a given prefix
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>>;
    
    /// Get base path of the storage
    fn base_path(&self) -> Option<PathBuf>;
}

/// Extension trait for JSON serialization/deserialization
#[async_trait]
pub trait JsonStorage: Storage {
    /// Store a serializable value at the specified key
    async fn put_json<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> StorageResult<()> {
        let json_data = serde_json::to_vec_pretty(value)
            .map_err(StorageError::SerializationError)?;
        self.put(key, &json_data).await
    }
    
    /// Retrieve and deserialize a value from the specified key
    async fn get_json<T: DeserializeOwned + Send>(&self, key: &str) -> StorageResult<T> {
        let data = self.get(key).await?;
        serde_json::from_slice(&data)
            .map_err(StorageError::SerializationError)
    }
}

// Implement JsonStorage for any type that implements Storage
#[async_trait]
impl<T: Storage> JsonStorage for T {}

// Module exports
pub mod file_storage;
pub use file_storage::FileStorage; 