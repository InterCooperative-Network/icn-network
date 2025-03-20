//! Storage system for the Intercooperative Network
//!
//! This crate provides the storage functionality for the ICN, including:
//! - Distributed storage
//! - Versioning
//! - Quota management
//! - Metrics collection
//! - Memory storage implementation

use std::error::Error;
use std::fmt;
use std::path::PathBuf;
use std::collections::HashMap;
use std::sync::Arc;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;
use thiserror::Error;

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
    QuotaConfig,
    QuotaError,
    UsageStats,
};

/// Storage-related errors
#[derive(Debug, Clone, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
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
        if err.is_data() {
            StorageError::DeserializationError(err.to_string())
        } else {
            StorageError::SerializationError(err.to_string())
        }
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
            .map_err(|e| StorageError::DeserializationError(e.to_string()))
    }

    /// Update a JSON value at the specified key with a transformation function
    async fn update_json<T, F>(&self, key: &str, update_fn: F) -> StorageResult<T>
    where
        T: DeserializeOwned + Serialize + Send,
        F: FnOnce(&mut T) -> StorageResult<()> + Send,
    {
        // Get existing data
        let data = match self.get(key).await {
            Ok(data) => data,
            Err(StorageError::KeyNotFound(_)) => {
                return Err(StorageError::KeyNotFound(key.to_string()))
            }
            Err(e) => return Err(e),
        };

        // Deserialize
        let mut value: T = serde_json::from_slice(&data)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))?;

        // Apply update
        update_fn(&mut value)?;

        // Serialize and store
        let json_data = serde_json::to_vec(&value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.put(key, &json_data).await?;

        Ok(value)
    }
}

// Implement JsonStorage for any type that implements Storage
impl<T: Storage + ?Sized> JsonStorage for T {}

/// A basic file system-based storage implementation 
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
        let base_path = self.base_path.clone();
        let prefix_path = self.get_full_path(prefix);
        
        if !prefix_path.exists() {
            return Ok(result);
        }
        
        let mut entries = tokio::fs::read_dir(&prefix_path).await
            .map_err(|e| StorageError::IoError(format!("Failed to read directory: {}", e)))?;
        
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| StorageError::IoError(format!("Failed to read directory entry: {}", e)))? {
                
            let path = entry.path();
            if let Ok(relative) = path.strip_prefix(&base_path) {
                if let Some(key) = relative.to_str() {
                    result.push(key.to_string());
                }
            }
        }
        
        Ok(result)
    }
    
    fn base_path(&self) -> Option<PathBuf> {
        Some(self.base_path.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs;
    
    #[tokio::test]
    async fn test_file_storage_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path().to_path_buf(), None);
        
        // Test put
        storage.put("test-key", b"test-data").await.unwrap();
        assert!(temp_dir.path().join("test-key").exists());
        
        // Test get
        let data = storage.get("test-key").await.unwrap();
        assert_eq!(data, b"test-data");
        
        // Test exists
        assert!(storage.exists("test-key").await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());
        
        // Test delete
        storage.delete("test-key").await.unwrap();
        assert!(!temp_dir.path().join("test-key").exists());
        
        // Test list
        storage.put("prefix/key1", b"data1").await.unwrap();
        storage.put("prefix/key2", b"data2").await.unwrap();
        let keys = storage.list("prefix").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix/key1".to_string()));
        assert!(keys.contains(&"prefix/key2".to_string()));
    }
    
    #[tokio::test]
    async fn test_json_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = FileStorage::new(temp_dir.path().to_path_buf(), None);
        
        #[derive(Debug, Serialize, Deserialize, PartialEq)]
        struct TestData {
            field1: String,
            field2: i32,
        }
        
        let test_data = TestData {
            field1: "test".to_string(),
            field2: 42,
        };
        
        // Test put_json
        storage.put_json("test-json", &test_data).await.unwrap();
        
        // Test get_json
        let retrieved: TestData = storage.get_json("test-json").await.unwrap();
        assert_eq!(retrieved, test_data);
        
        // Test update_json
        let updated: TestData = storage.update_json("test-json", |data| {
            data.field2 = 43;
            Ok(())
        }).await.unwrap();
        
        assert_eq!(updated.field2, 43);
        assert_eq!(updated.field1, "test");
    }
} 