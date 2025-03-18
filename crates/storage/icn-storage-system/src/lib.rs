/// Storage system for the ICN Network
///
/// This crate provides a distributed storage system for the ICN Network,
/// supporting encrypted, versioned, and permission-controlled storage.

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
    async fn put_bytes(&self, key: &str, value: &[u8]) -> Result<()>;
    
    /// Get a value by key
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>>;
    
    /// Delete a value by key
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool>;
    
    /// List all keys with the given prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>>;
    
    /// Clear all stored data
    async fn clear(&self) -> Result<()>;
}

/// Extension methods for Storage trait
pub trait StorageExt: Storage {
    /// Store a serializable value with the given key
    async fn put<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| Error::serialization(format!("Failed to serialize value: {}", e)))?;
        self.put_bytes(key, &serialized).await
    }
    
    /// Get a deserialized value by key
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>> {
        if let Some(data) = self.get_bytes(key).await? {
            let value = serde_json::from_slice(&data)
                .map_err(|e| Error::serialization(format!("Failed to deserialize value: {}", e)))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }
}

// Implement StorageExt for all Storage implementors
impl<T: Storage + ?Sized> StorageExt for T {}

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
            .map_err(|e| Error::internal(format!("Failed to create storage directory: {}", e)))?;
        
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
    async fn put_bytes(&self, key: &str, value: &[u8]) -> Result<()> {
        // Update cache
        self.cache.write().await.insert(key.to_string(), value.to_vec());
        
        // Write to file
        let path = self.get_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await
                .map_err(|e| Error::internal(format!("Failed to create directory: {}", e)))?;
        }
        
        fs::write(&path, value).await
            .map_err(|e| Error::internal(format!("Failed to write file: {}", e)))?;
        
        if self.options.sync_writes {
            // TODO: Implement fsync
        }
        
        Ok(())
    }
    
    async fn get_bytes(&self, key: &str) -> Result<Option<Vec<u8>>> {
        // Check cache first
        if let Some(cached) = self.cache.read().await.get(key) {
            return Ok(Some(cached.clone()));
        }
        
        // Read from file
        let path = self.get_path(key);
        let data = match fs::read(&path).await {
            Ok(data) => data,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(Error::internal(format!("Failed to read file: {}", e))),
        };
        
        // Update cache
        self.cache.write().await.insert(key.to_string(), data.clone());
        
        Ok(Some(data))
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        // Remove from cache
        self.cache.write().await.remove(key);
        
        // Remove file
        let path = self.get_path(key);
        match fs::remove_file(path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Error::internal(format!("Failed to delete file: {}", e))),
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
        
        // Check cache first
        for key in self.cache.read().await.keys() {
            if key.starts_with(prefix) {
                keys.push(key.clone());
            }
        }
        
        // Check file system
        let prefix_path = self.get_path(prefix);
        let prefix_dir = if prefix.ends_with('/') {
            prefix_path
        } else {
            prefix_path.parent().unwrap_or(&self.options.base_dir).to_path_buf()
        };
        
        if prefix_dir.exists() {
            let mut stack = vec![prefix_dir.clone()];
            
            while let Some(dir) = stack.pop() {
                if let Ok(mut entries) = fs::read_dir(&dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let path = entry.path();
                        
                        if path.is_dir() {
                            stack.push(path);
                        } else if let Ok(rel_path) = path.strip_prefix(&self.options.base_dir) {
                            if let Some(key) = rel_path.to_str() {
                                if key.starts_with(prefix) && !keys.contains(&key.to_string()) {
                                    keys.push(key.to_string());
                                }
                            }
                        }
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
            .map_err(|e| Error::internal(format!("Failed to clear storage: {}", e)))?;
            
        // Recreate base directory
        fs::create_dir_all(&self.options.base_dir).await
            .map_err(|e| Error::internal(format!("Failed to create storage directory: {}", e)))?;
            
        Ok(())
    }
}

/// Create a new storage instance with the given options
pub async fn create_storage(options: StorageOptions) -> Result<Arc<FileStorage>> {
    let storage = FileStorage::new(options).await?;
    Ok(Arc::new(storage))
}

/// Storage service for the ICN Network
pub struct StorageService {
    /// The base path for the storage
    base_path: std::path::PathBuf,
}

impl StorageService {
    /// Create a new storage service
    pub fn new<P: AsRef<std::path::Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }
    
    /// Get the base path for the storage
    pub fn base_path(&self) -> &std::path::Path {
        &self.base_path
    }
}

/// Mock implementation for testing
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
    
    #[test]
    fn test_create_storage_service() {
        let service = StorageService::new("/tmp/icn-storage");
        assert_eq!(service.base_path().to_str().unwrap(), "/tmp/icn-storage");
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
        
        // Use the extension trait methods
        storage.put("test_key", &test_data).await.unwrap();
        
        let retrieved: TestData = storage.get("test_key").await.unwrap().unwrap();
        assert_eq!(retrieved, test_data);
        
        // Test exists
        assert!(storage.exists("test_key").await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());
        
        // Test delete
        storage.delete("test_key").await.unwrap();
        assert!(!storage.exists("test_key").await.unwrap());
        
        // Test raw bytes
        let raw_data = b"raw data test";
        storage.put_bytes("raw_key", raw_data).await.unwrap();
        let retrieved_raw = storage.get_bytes("raw_key").await.unwrap().unwrap();
        assert_eq!(retrieved_raw, raw_data);
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