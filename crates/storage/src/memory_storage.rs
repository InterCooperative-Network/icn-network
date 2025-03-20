use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

use crate::{Storage, StorageResult, StorageError};

/// In-memory storage implementation
pub struct MemoryStorage {
    data: RwLock<HashMap<String, Vec<u8>>>,
}

impl MemoryStorage {
    /// Create a new memory storage instance
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
        let mut storage = self.data.write().await;
        storage.insert(key.to_string(), data.to_vec());
        Ok(())
    }
    
    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        let storage = self.data.read().await;
        storage.get(key)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
    }
    
    async fn delete(&self, key: &str) -> StorageResult<()> {
        let mut storage = self.data.write().await;
        if storage.remove(key).is_none() {
            return Err(StorageError::KeyNotFound(key.to_string()));
        }
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> StorageResult<bool> {
        let storage = self.data.read().await;
        Ok(storage.contains_key(key))
    }
    
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let storage = self.data.read().await;
        Ok(storage.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }
    
    fn base_path(&self) -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JsonStorage;
    use serde::{Deserialize, Serialize};
    
    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemoryStorage::new();
        
        // Test put
        storage.put("test-key", b"test-data").await.unwrap();
        
        // Test get
        let data = storage.get("test-key").await.unwrap();
        assert_eq!(data, b"test-data");
        
        // Test exists
        assert!(storage.exists("test-key").await.unwrap());
        assert!(!storage.exists("nonexistent").await.unwrap());
        
        // Test delete
        storage.delete("test-key").await.unwrap();
        assert!(!storage.exists("test-key").await.unwrap());
        
        // Test list
        storage.put("prefix/key1", b"data1").await.unwrap();
        storage.put("prefix/key2", b"data2").await.unwrap();
        let keys = storage.list("prefix").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix/key1".to_string()));
        assert!(keys.contains(&"prefix/key2".to_string()));
    }
    
    #[tokio::test]
    async fn test_memory_storage_json() {
        let storage = MemoryStorage::new();
        
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
    
    #[tokio::test]
    async fn test_memory_storage_errors() {
        let storage = MemoryStorage::new();
        
        // Test get nonexistent key
        let result = storage.get("nonexistent").await;
        assert!(matches!(result, Err(StorageError::KeyNotFound(_))));
        
        // Test delete nonexistent key
        let result = storage.delete("nonexistent").await;
        assert!(matches!(result, Err(StorageError::KeyNotFound(_))));
    }
} 