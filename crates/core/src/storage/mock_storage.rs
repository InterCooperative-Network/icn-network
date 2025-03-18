//! Mock storage implementation for testing
//!
//! This module provides a mock implementation of the Storage trait
//! for use in tests.

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use super::{Storage, StorageResult, StorageError};

/// A memory-based mock storage implementation for testing
pub struct MockStorage {
    /// In-memory data store
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    /// Base path (optional, for compatibility)
    base_path: Option<PathBuf>,
}

impl MockStorage {
    /// Create a new MockStorage
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            base_path: None,
        }
    }
    
    /// Create a new MockStorage with a base path
    pub fn with_base_path(base_path: PathBuf) -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            base_path: Some(base_path),
        }
    }
    
    /// Set predefined data
    pub fn with_data(self, data: HashMap<String, Vec<u8>>) -> Self {
        // Clone self first to avoid borrowing issues
        let mut result = self;
        let mut store = result.data.write().unwrap();
        *store = data;
        drop(store); // Explicitly drop the write guard before returning
        result
    }
    
    /// Clear all data
    pub fn clear(&self) {
        let mut store = self.data.write().unwrap();
        store.clear();
    }
}

#[async_trait]
impl Storage for MockStorage {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
        let mut store = self.data.write().unwrap();
        store.insert(key.to_string(), data.to_vec());
        Ok(())
    }
    
    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        let store = self.data.read().unwrap();
        store.get(key)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
    }
    
    async fn delete(&self, key: &str) -> StorageResult<()> {
        let mut store = self.data.write().unwrap();
        store.remove(key);
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let store = self.data.read().unwrap();
        Ok(store.contains_key(key))
    }
    
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let store = self.data.read().unwrap();
        let keys = store.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
    
    fn base_path(&self) -> Option<PathBuf> {
        self.base_path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_put_and_get() {
        let storage = MockStorage::new();
        let key = "test_key";
        let data = b"test_data".to_vec();
        
        storage.put(key, &data).await.unwrap();
        let retrieved = storage.get(key).await.unwrap();
        
        assert_eq!(retrieved, data);
    }
    
    #[tokio::test]
    async fn test_delete() {
        let storage = MockStorage::new();
        let key = "test_key";
        let data = b"test_data".to_vec();
        
        storage.put(key, &data).await.unwrap();
        assert!(storage.exists(key).await.unwrap());
        
        storage.delete(key).await.unwrap();
        assert!(!storage.exists(key).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_list() {
        let storage = MockStorage::new();
        
        storage.put("prefix1_key1", b"data1").await.unwrap();
        storage.put("prefix1_key2", b"data2").await.unwrap();
        storage.put("prefix2_key3", b"data3").await.unwrap();
        
        let prefix1_keys = storage.list("prefix1_").await.unwrap();
        assert_eq!(prefix1_keys.len(), 2);
        assert!(prefix1_keys.contains(&"prefix1_key1".to_string()));
        assert!(prefix1_keys.contains(&"prefix1_key2".to_string()));
        
        let prefix2_keys = storage.list("prefix2_").await.unwrap();
        assert_eq!(prefix2_keys.len(), 1);
        assert!(prefix2_keys.contains(&"prefix2_key3".to_string()));
    }
} 