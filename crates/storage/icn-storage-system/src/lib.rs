//! Storage system for ICN
//!
//! This crate provides a unified storage interface for persistent data storage
//! across the ICN network components.

use async_trait::async_trait;
use icn_common::{Error, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::sync::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Storage options for configuring storage behavior
#[derive(Debug, Clone)]
pub struct StorageOptions {
    /// Base directory for storage
    pub base_dir: PathBuf,
    /// Whether to sync writes to disk immediately
    pub sync_writes: bool,
    /// Whether to compress stored data
    pub compress: bool,
}

impl Default for StorageOptions {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from("data"),
            sync_writes: true,
            compress: false,
        }
    }
}

/// Storage interface for persistent data
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a value with the given key
    async fn put<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()>;
    
    /// Get a value by key
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>>;
    
    /// Delete a value by key
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool>;
    
    /// List all keys with the given prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    
    /// Clear all stored data
    async fn clear(&self) -> Result<()>;
}

/// File-based storage implementation
pub struct FileStorage {
    options: StorageOptions,
    cache: RwLock<HashMap<String, Vec<u8>>>,
}

impl FileStorage {
    /// Create a new file storage instance
    pub async fn new(options: StorageOptions) -> Result<Self> {
        // Create base directory if it doesn't exist
        fs::create_dir_all(&options.base_dir).await
            .map_err(|e| Error::configuration(format!("Failed to create storage directory: {}", e)))?;
        
        Ok(Self {
            options,
            cache: RwLock::new(HashMap::new()),
        })
    }
    
    /// Get the full path for a key
    fn get_path(&self, key: &str) -> PathBuf {
        self.options.base_dir.join(key)
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn put<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = if self.options.compress {
            // TODO: Implement compression
            serde_json::to_vec(value)
                .map_err(|e| Error::serialization(format!("Failed to serialize value: {}", e)))?
        } else {
            serde_json::to_vec(value)
                .map_err(|e| Error::serialization(format!("Failed to serialize value: {}", e)))?
        };
        
        // Update cache
        self.cache.write().await.insert(key.to_string(), serialized.clone());
        
        // Write to file
        let path = self.get_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| Error::configuration(format!("Failed to create directory: {}", e)))?;
        }
        
        fs::write(&path, serialized).await
            .map_err(|e| Error::configuration(format!("Failed to write file: {}", e)))?;
        
        if self.options.sync_writes {
            // TODO: Implement fsync
        }
        
        Ok(())
    }
    
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        // Check cache first
        if let Some(cached) = self.cache.read().await.get(key) {
            return Ok(Some(serde_json::from_slice(cached)
                .map_err(|e| Error::serialization(format!("Failed to deserialize value: {}", e)))?));
        }
        
        // Read from file
        let path = self.get_path(key);
        let data = match fs::read(&path).await {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Error::configuration(format!("Failed to read file: {}", e))),
        };
        
        // Update cache
        self.cache.write().await.insert(key.to_string(), data.clone());
        
        // Deserialize
        let value = serde_json::from_slice(&data)
            .map_err(|e| Error::serialization(format!("Failed to deserialize value: {}", e)))?;
            
        Ok(Some(value))
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        // Remove from cache
        self.cache.write().await.remove(key);
        
        // Remove file
        let path = self.get_path(key);
        match fs::remove_file(path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::configuration(format!("Failed to delete file: {}", e))),
        }
    }
    
    async fn exists(&self, key: &str) -> Result<bool> {
        // Check cache first
        if self.cache.read().await.contains_key(key) {
            return Ok(true);
        }
        
        // Check file system
        let path = self.get_path(key);
        Ok(path.exists())
    }
    
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();
        let base_path = self.get_path(prefix);
        
        // List files in directory
        let mut entries = fs::read_dir(&self.options.base_dir).await
            .map_err(|e| Error::configuration(format!("Failed to read directory: {}", e)))?;
            
        while let Some(entry) = entries.next_entry().await
            .map_err(|e| Error::configuration(format!("Failed to read directory entry: {}", e)))? {
            
            if let Ok(path) = entry.path().strip_prefix(&self.options.base_dir) {
                if let Some(key) = path.to_str() {
                    if key.starts_with(prefix) {
                        keys.push(key.to_string());
                    }
                }
            }
        }
        
        Ok(keys)
    }
    
    async fn clear(&self) -> Result<()> {
        // Clear cache
        self.cache.write().await.clear();
        
        // Remove all files in base directory
        fs::remove_dir_all(&self.options.base_dir).await
            .map_err(|e| Error::configuration(format!("Failed to clear storage: {}", e)))?;
            
        // Recreate base directory
        fs::create_dir_all(&self.options.base_dir).await
            .map_err(|e| Error::configuration(format!("Failed to create storage directory: {}", e)))?;
            
        Ok(())
    }
}

/// Create a new storage instance with the given options
pub async fn create_storage(options: StorageOptions) -> Result<Arc<dyn Storage>> {
    let storage = FileStorage::new(options).await?;
    Ok(Arc::new(storage))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        field1: String,
        field2: i32,
    }
    
    #[tokio::test]
    async fn test_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };
        
        let storage = FileStorage::new(options).await.unwrap();
        
        // Test put and get
        let test_data = TestData {
            field1: "test".to_string(),
            field2: 42,
        };
        
        storage.put("test_key", &test_data).await.unwrap();
        
        let retrieved: TestData = storage.get("test_key").await.unwrap().unwrap();
        assert_eq!(retrieved, test_data);
        
        // Test exists
        assert!(storage.exists("test_key").await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());
        
        // Test delete
        storage.delete("test_key").await.unwrap();
        assert!(!storage.exists("test_key").await.unwrap());
        
        // Test clear
        storage.put("key1", &test_data).await.unwrap();
        storage.put("key2", &test_data).await.unwrap();
        storage.clear().await.unwrap();
        assert!(!storage.exists("key1").await.unwrap());
        assert!(!storage.exists("key2").await.unwrap());
    }
    
    #[tokio::test]
    async fn test_list_keys() {
        let temp_dir = tempdir().unwrap();
        let options = StorageOptions {
            base_dir: temp_dir.path().to_path_buf(),
            sync_writes: true,
            compress: false,
        };
        
        let storage = FileStorage::new(options).await.unwrap();
        
        let test_data = TestData {
            field1: "test".to_string(),
            field2: 42,
        };
        
        storage.put("prefix1/key1", &test_data).await.unwrap();
        storage.put("prefix1/key2", &test_data).await.unwrap();
        storage.put("prefix2/key3", &test_data).await.unwrap();
        
        let keys = storage.list_keys("prefix1/").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix1/key1".to_string()));
        assert!(keys.contains(&"prefix1/key2".to_string()));
        
        let keys = storage.list_keys("prefix2/").await.unwrap();
        assert_eq!(keys.len(), 1);
        assert!(keys.contains(&"prefix2/key3".to_string()));
    }
}