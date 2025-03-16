use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use std::path::PathBuf;

/// A mock storage implementation for testing
#[derive(Default)]
pub struct MockStorage {
    data: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl MockStorage {
    /// Create a new mock storage
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[async_trait]
impl icn_core::storage::Storage for MockStorage {
    async fn get(&self, key: &str) -> Result<Vec<u8>, icn_core::storage::StorageError> {
        let data = self.data.lock().unwrap();
        match data.get(key) {
            Some(value) => Ok(value.clone()),
            None => Err(icn_core::storage::StorageError::KeyNotFound(key.to_string())),
        }
    }

    async fn put(&self, key: &str, data: &[u8]) -> Result<(), icn_core::storage::StorageError> {
        let mut data_map = self.data.lock().unwrap();
        data_map.insert(key.to_string(), data.to_vec());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), icn_core::storage::StorageError> {
        let mut data = self.data.lock().unwrap();
        data.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, icn_core::storage::StorageError> {
        let data = self.data.lock().unwrap();
        Ok(data.contains_key(key))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, icn_core::storage::StorageError> {
        let data = self.data.lock().unwrap();
        let keys: Vec<String> = data
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }

    fn base_path(&self) -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_storage() {
        let storage = MockStorage::new();
        
        // Test put and get
        storage.put("test_key", b"test_value").await.unwrap();
        let value = storage.get("test_key").await.unwrap();
        assert_eq!(value, b"test_value".to_vec());
        
        // Test exists
        assert!(storage.exists("test_key").await.unwrap());
        assert!(!storage.exists("nonexistent_key").await.unwrap());
        
        // Test delete
        storage.delete("test_key").await.unwrap();
        assert!(!storage.exists("test_key").await.unwrap());
        
        // Test keys with prefix
        storage.put("prefix1_key1", b"value1").await.unwrap();
        storage.put("prefix1_key2", b"value2").await.unwrap();
        storage.put("prefix2_key1", b"value3").await.unwrap();
        
        let keys = storage.list("prefix1_").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"prefix1_key1".to_string()));
        assert!(keys.contains(&"prefix1_key2".to_string()));
    }
} 