use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;

use super::{Storage, StorageResult, StorageError};

/// In-memory implementation of Storage for testing
pub struct MemoryStorage {
    data: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl MemoryStorage {
    /// Create a new empty memory storage
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
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
        let result: Vec<String> = store.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(result)
    }

    fn base_path(&self) -> Option<PathBuf> {
        None // Memory storage has no base path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::versioning::{VersioningManager, VersionInfo};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_memory_storage_basic_operations() {
        let storage = MemoryStorage::new();
        
        // Test put and get
        let key = "test-key";
        let data = b"test data";
        
        storage.put(key, data).await.unwrap();
        assert!(storage.exists(key).await.unwrap());
        
        let retrieved = storage.get(key).await.unwrap();
        assert_eq!(retrieved, data);
        
        // Test list
        let key2 = "test-key2";
        storage.put(key2, b"more data").await.unwrap();
        
        let keys = storage.list("test-").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&key.to_string()));
        assert!(keys.contains(&key2.to_string()));
        
        // Test delete
        storage.delete(key).await.unwrap();
        assert!(!storage.exists(key).await.unwrap());
        
        // Getting a deleted key should fail
        let result = storage.get(key).await;
        assert!(result.is_err());
        
        // Deleting a non-existent key should fail
        let result = storage.delete("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_versioning_with_memory_storage() {
        let storage = Arc::new(MemoryStorage::new());
        let versioning = VersioningManager::new(Arc::clone(&storage), 3);
        
        // Generate a test key and version ID
        let key = "versioned-key";
        let version_id = versioning.generate_version_id();
        let version_key = versioning.create_version_storage_key(key, &version_id);
        
        // Store version data
        let data = b"Version 1 data";
        storage.put(&version_key, data).await.unwrap();
        
        // Create version info
        let version_info = VersionInfo {
            version_id: version_id.clone(),
            created_at: 1000,
            size_bytes: data.len() as u64,
            metadata: HashMap::new(),
            storage_key: version_key.clone(),
            content_hash: "hash1".to_string(),
            created_by: "test-user".to_string(),
            comment: Some("Initial version".to_string()),
        };
        
        // Initialize versioning 
        versioning.init_versioning(key, None, Some(version_info)).await.unwrap();
        
        // Get history and verify
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.current_version_id, Some(version_id));
        
        // Clean up
        versioning.delete_all_versions(key).await.unwrap();
    }
} 