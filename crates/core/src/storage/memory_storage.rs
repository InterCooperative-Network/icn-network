use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;
use tokio::sync::RwLock;

use super::{Storage, StorageError, StorageResult};

/// In-memory storage implementation for testing
#[derive(Debug, Clone)]
pub struct MemoryStorage {
    /// In-memory data store
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl MemoryStorage {
    /// Create a new empty memory storage
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new memory storage with initial data
    pub fn with_data(data: HashMap<String, Vec<u8>>) -> Self {
        Self {
            data: Arc::new(RwLock::new(data)),
        }
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    /// Store data at the specified key
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
        let mut store = self.data.write().await;
        store.insert(key.to_string(), data.to_vec());
        Ok(())
    }
    
    /// Retrieve data from the specified key
    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        let store = self.data.read().await;
        store.get(key)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
    }
    
    /// Delete data at the specified key
    async fn delete(&self, key: &str) -> StorageResult<()> {
        let mut store = self.data.write().await;
        store.remove(key);
        Ok(())
    }
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let store = self.data.read().await;
        Ok(store.contains_key(key))
    }
    
    /// List all keys with a given prefix
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let store = self.data.read().await;
        let keys = store.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
    
    /// Get base path of the storage (None for memory storage)
    fn base_path(&self) -> Option<PathBuf> {
        None
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::JsonStorage;

    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemoryStorage::new();
        
        // Test put and get
        storage.put("test1", b"Hello World").await.unwrap();
        let data = storage.get("test1").await.unwrap();
        assert_eq!(data, b"Hello World");
        
        // Test overwrite
        storage.put("test1", b"Updated Content").await.unwrap();
        let data = storage.get("test1").await.unwrap();
        assert_eq!(data, b"Updated Content");
        
        // Test list
        storage.put("prefix/key1", b"Value 1").await.unwrap();
        storage.put("prefix/key2", b"Value 2").await.unwrap();
        storage.put("other/key3", b"Value 3").await.unwrap();
        
        let keys = storage.list("prefix/").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix/key1".to_string()));
        assert!(keys.contains(&"prefix/key2".to_string()));
        
        // Test exists
        assert!(storage.exists("test1").await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());
        
        // Test delete
        storage.delete("test1").await.unwrap();
        assert!(!storage.exists("test1").await.unwrap());
        assert!(storage.get("test1").await.is_err());
    }
    
    #[tokio::test]
    async fn test_json_storage() {
        let storage = MemoryStorage::new();
        
        // Test JSON serialization/deserialization
        let data = HashMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]);
        
        storage.put_json("json_test", &data).await.unwrap();
        
        let retrieved: HashMap<String, String> = storage.get_json("json_test").await.unwrap();
        assert_eq!(retrieved, data);
    }
} 