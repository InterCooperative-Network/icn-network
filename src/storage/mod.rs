use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

pub mod versioning;
pub mod metrics;
pub mod quota;
pub mod memory_storage;

// Re-export commonly used types and functions
pub use versioning::{
    VersionInfo,
    VersionHistory,
    VersioningManager,
    VersioningError,
};

pub use metrics::{
    StorageMetrics,
    MetricsSnapshot,
    MetricsTimer,
    OperationType,
};

pub use quota::{
    QuotaManager,
    OperationScheduler,
    StorageQuota,
    QuotaOperation,
    QuotaEntityType,
    QuotaCheckResult,
    QuotaUtilization,
};

/// Storage-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Not a directory: {0}")]
    NotADirectory(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Insufficient resources: {0}")]
    InsufficientResources(String),
    
    #[error("Unexpected error: {0}")]
    Other(String),
}

impl From<std::io::Error> for StorageError {
    fn from(err: std::io::Error) -> Self {
        StorageError::IoError(err.to_string())
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(err: serde_json::Error) -> Self {
        StorageError::SerializationError(err.to_string())
    }
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage options
#[derive(Debug, Clone)]
pub struct StorageOptions {
    pub sync_write: bool,
    pub create_dirs: bool,
    pub use_cache: bool,
    pub encryption_enabled: bool,
    pub max_key_size: usize,
    pub max_value_size: usize,
}

impl Default for StorageOptions {
    fn default() -> Self {
        StorageOptions {
            sync_write: true,
            create_dirs: true,
            use_cache: true,
            encryption_enabled: false,
            max_key_size: 1024,  // 1KB
            max_value_size: 10 * 1024 * 1024, // 10MB
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
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.put(key, &json_data).await
    }
    
    /// Retrieve and deserialize a value from the specified key
    async fn get_json<T: DeserializeOwned + Send>(&self, key: &str) -> StorageResult<T> {
        let data = self.get(key).await?;
        serde_json::from_slice(&data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))
    }
}

// Implement JsonStorage for any type that implements Storage
#[async_trait]
impl<T: Storage + ?Sized> JsonStorage for T {}

// A basic file system-based storage implementation 
pub struct FileStorage {
    base_path: PathBuf,
    options: StorageOptions,
}

impl FileStorage {
    pub fn new(base_path: PathBuf, options: Option<StorageOptions>) -> Self {
        let options = options.unwrap_or_default();
        
        // Create the directory if it doesn't exist
        if options.create_dirs {
            std::fs::create_dir_all(&base_path).ok();
        }
        
        FileStorage {
            base_path,
            options,
        }
    }
    
    fn get_full_path(&self, key: &str) -> PathBuf {
        let mut path = self.base_path.clone();
        path.push(key);
        path
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
        let path = self.get_full_path(key);
        
        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            if self.options.create_dirs {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| StorageError::IoError(format!("Failed to create directory: {}", e)))?;
            }
        }
        
        // Write the data
        tokio::fs::write(&path, data).await
            .map_err(|e| StorageError::IoError(format!("Failed to write data: {}", e)))?;
        
        Ok(())
    }
    
    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        let path = self.get_full_path(key);
        
        if !path.exists() {
            return Err(StorageError::KeyNotFound(key.to_string()));
        }
        
        tokio::fs::read(&path).await
            .map_err(|e| StorageError::IoError(format!("Failed to read data: {}", e)))
    }
    
    async fn delete(&self, key: &str) -> StorageResult<()> {
        let path = self.get_full_path(key);
        
        if !path.exists() {
            return Err(StorageError::KeyNotFound(key.to_string()));
        }
        
        tokio::fs::remove_file(&path).await
            .map_err(|e| StorageError::IoError(format!("Failed to delete file: {}", e)))?;
        
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let path = self.get_full_path(key);
        Ok(path.exists())
    }
    
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let mut result = Vec::new();
        let base_dir = self.base_path.clone();
        let prefix_path = self.get_full_path(prefix);
        
        // The directory might not exist yet
        if !prefix_path.exists() {
            return Ok(vec![]);
        }
        
        // Make sure it's a directory
        if prefix_path.is_file() {
            return Err(StorageError::NotADirectory(prefix.to_string()));
        }
        
        let mut entries = tokio::fs::read_dir(prefix_path).await
            .map_err(|e| StorageError::IoError(format!("Failed to list directory: {}", e)))?;
        
        while let Ok(Some(entry)) = entries.next_entry().await {
            let path = entry.path();
            if let Ok(rel_path) = path.strip_prefix(&base_dir) {
                if let Some(path_str) = rel_path.to_str() {
                    result.push(path_str.to_string());
                }
            }
        }
        
        Ok(result)
    }
    
    fn base_path(&self) -> Option<PathBuf> {
        Some(self.base_path.clone())
    }
}

/// In-memory storage implementation for testing
#[cfg(test)]
pub mod memory_storage {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::RwLock;

    pub struct MemoryStorage {
        data: RwLock<HashMap<String, Vec<u8>>>,
    }

    impl MemoryStorage {
        pub fn new() -> Self {
            MemoryStorage {
                data: RwLock::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl Storage for MemoryStorage {
        async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
            let mut store = self.data.write().await;
            store.insert(key.to_string(), data.to_vec());
            Ok(())
        }
        
        async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
            let store = self.data.read().await;
            store.get(key)
                .cloned()
                .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
        }
        
        async fn delete(&self, key: &str) -> StorageResult<()> {
            let mut store = self.data.write().await;
            if store.remove(key).is_none() {
                return Err(StorageError::KeyNotFound(key.to_string()));
            }
            Ok(())
        }
        
        async fn exists(&self, key: &str) -> StorageResult<bool> {
            let store = self.data.read().await;
            Ok(store.contains_key(key))
        }
        
        async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
            let store = self.data.read().await;
            let keys: Vec<String> = store.keys()
                .filter(|k| k.starts_with(prefix))
                .cloned()
                .collect();
            Ok(keys)
        }
        
        fn base_path(&self) -> Option<PathBuf> {
            None
        }
    }
} 