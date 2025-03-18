use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use std::fmt;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::{Storage, StorageError, JsonStorage};

/// Information about a specific version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Unique version identifier
    pub version_id: String,
    /// Timestamp when this version was created
    pub created_at: u64,
    /// Size in bytes of this version
    pub size_bytes: u64,
    /// Optional metadata associated with this version
    pub metadata: HashMap<String, String>,
    /// Storage key where the version data is stored
    pub storage_key: String,
    /// Content hash of the version data
    pub content_hash: String,
    /// Identity of who created this version
    pub created_by: String,
    /// Optional comment about this version
    pub comment: Option<String>,
}

/// Track version history for a key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    /// Key this history is for
    pub key: String,
    /// Map of version IDs to version info
    pub versions: HashMap<String, VersionInfo>,
    /// Current version ID
    pub current_version_id: Option<String>,
    /// Total size of all versions
    pub total_size_bytes: u64,
    /// Maximum number of versions to keep
    pub max_versions: usize,
}

impl VersionHistory {
    /// Create a new version history for a key
    pub fn new(key: &str, max_versions: usize) -> Self {
        Self {
            key: key.to_string(),
            versions: HashMap::new(),
            current_version_id: None,
            total_size_bytes: 0,
            max_versions: max_versions.max(1), // Ensure at least 1 version
        }
    }

    /// Add a version to the history
    pub fn add_version(&mut self, version: VersionInfo) -> Option<VersionInfo> {
        // Update total size
        self.total_size_bytes += version.size_bytes;

        // Set as current version if none exists
        if self.current_version_id.is_none() {
            self.current_version_id = Some(version.version_id.clone());
        }

        // Add to versions map
        let version_id = version.version_id.clone();
        self.versions.insert(version_id.clone(), version);

        // Prune oldest versions if we exceed max_versions
        if self.versions.len() > self.max_versions {
            // Find oldest version (by created_at) that's not the current version
            let current_version_id = self.current_version_id.clone();
            
            let oldest_version_id = self.versions.iter()
                .filter(|(id, _)| Some(*id) != current_version_id.as_ref())
                .min_by_key(|(_, v)| v.created_at)
                .map(|(id, _)| id.clone());

            // Remove oldest version if found
            if let Some(id) = oldest_version_id {
                if let Some(oldest) = self.versions.remove(&id) {
                    self.total_size_bytes = self.total_size_bytes.saturating_sub(oldest.size_bytes);
                    return Some(oldest);
                }
            }
        }

        None
    }

    /// Get a specific version
    pub fn get_version(&self, version_id: &str) -> Option<&VersionInfo> {
        self.versions.get(version_id)
    }

    /// Set the current version
    pub fn set_current_version(&mut self, version_id: &str) -> bool {
        if self.versions.contains_key(version_id) {
            self.current_version_id = Some(version_id.to_string());
            true
        } else {
            false
        }
    }

    /// Get the current version
    pub fn get_current_version(&self) -> Option<&VersionInfo> {
        self.current_version_id.as_ref().and_then(|id| self.versions.get(id))
    }

    /// Get the total size of all versions
    pub fn total_size(&self) -> u64 {
        self.total_size_bytes
    }

    /// Get the number of versions
    pub fn version_count(&self) -> usize {
        self.versions.len()
    }
}

/// Errors related to versioning operations
#[derive(Debug, Error)]
pub enum VersioningError {
    /// Storage error
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    /// Version not found
    #[error("Version not found: {0}")]
    VersionNotFound(String),

    /// Key not found
    #[error("Key not found: {0}")]
    KeyNotFound(String),

    /// Invalid version
    #[error("Invalid version: {0}")]
    InvalidVersion(String),

    /// Version conflict
    #[error("Version conflict: {0}")]
    VersionConflict(String),

    /// Other errors
    #[error("Error: {0}")]
    Other(String),
}

/// Result type for versioning operations
pub type VersioningResult<T> = Result<T, VersioningError>;

/// Manager for versioning storage objects
pub struct VersioningManager {
    /// Storage for version data and metadata
    storage: Arc<dyn Storage>,
    /// Version histories by key
    histories: RwLock<HashMap<String, VersionHistory>>,
    /// Maximum number of versions to keep per key
    max_versions: usize,
}

impl VersioningManager {
    /// Create a new versioning manager
    pub fn new(storage: Arc<dyn Storage>, max_versions: usize) -> Self {
        Self {
            storage,
            histories: RwLock::new(HashMap::new()),
            max_versions: max_versions.max(1), // Ensure at least 1 version
        }
    }

    /// Generate a new version ID
    pub fn generate_version_id(&self) -> String {
        Uuid::new_v4().to_string()
    }

    /// Create a storage key for a version
    pub fn create_version_storage_key(&self, key: &str, version_id: &str) -> String {
        format!("versions/{}/{}", key, version_id)
    }

    /// Initialize versioning for a key with the first version
    pub async fn init_versioning(
        &self,
        key: &str,
        version_id: &str,
        version_info: VersionInfo,
    ) -> VersioningResult<()> {
        let mut histories = self.histories.write().await;
        
        // Create a new version history if it doesn't exist
        if !histories.contains_key(key) {
            let mut history = VersionHistory::new(key, self.max_versions);
            history.add_version(version_info);
            histories.insert(key.to_string(), history);
            
            // Store the history
            self.store_json(&format!("version_history/{}", key), &histories[key])
                .await?;
            
            Ok(())
        } else {
            Err(VersioningError::VersionConflict(format!(
                "Versioning already initialized for key: {}",
                key
            )))
        }
    }

    /// Create a new version for a key
    pub async fn create_version(
        &self,
        key: &str,
        version_id: &str,
        version_info: VersionInfo,
    ) -> VersioningResult<()> {
        let mut histories = self.histories.write().await;
        
        // Get or load the version history
        if !histories.contains_key(key) {
            match self.get_json::<VersionHistory>(&format!("version_history/{}", key)).await {
                Ok(history) => {
                    histories.insert(key.to_string(), history);
                },
                Err(VersioningError::KeyNotFound(_)) => {
                    return Err(VersioningError::KeyNotFound(key.to_string()));
                },
                Err(e) => return Err(e),
            }
        }
        
        // Get the history and add the new version
        if let Some(history) = histories.get_mut(key) {
            // Add the version
            let removed = history.add_version(version_info);
            
            // Store updated history
            self.store_json(&format!("version_history/{}", key), history)
                .await?;
            
            // Clean up removed version if necessary
            if let Some(removed_version) = removed {
                // Optional: Delete the storage for the removed version
                let _ = self.storage.delete(&removed_version.storage_key).await;
            }
            
            Ok(())
        } else {
            Err(VersioningError::KeyNotFound(key.to_string()))
        }
    }

    /// Get a specific version
    pub async fn get_version(
        &self,
        key: &str,
        version_id: &str,
    ) -> VersioningResult<VersionInfo> {
        let mut histories = self.histories.write().await;
        
        // Get or load the version history
        if !histories.contains_key(key) {
            match self.get_json::<VersionHistory>(&format!("version_history/{}", key)).await {
                Ok(history) => {
                    histories.insert(key.to_string(), history);
                },
                Err(VersioningError::KeyNotFound(_)) => {
                    return Err(VersioningError::KeyNotFound(key.to_string()));
                },
                Err(e) => return Err(e),
            }
        }
        
        // Get the version info
        if let Some(history) = histories.get(key) {
            if let Some(version) = history.get_version(version_id) {
                Ok(version.clone())
            } else {
                Err(VersioningError::VersionNotFound(version_id.to_string()))
            }
        } else {
            Err(VersioningError::KeyNotFound(key.to_string()))
        }
    }

    /// Get version history for a key
    pub async fn get_version_history(
        &self,
        key: &str,
    ) -> VersioningResult<VersionHistory> {
        let mut histories = self.histories.write().await;
        
        // Get or load the version history
        if !histories.contains_key(key) {
            match self.get_json::<VersionHistory>(&format!("version_history/{}", key)).await {
                Ok(history) => {
                    histories.insert(key.to_string(), history);
                },
                Err(VersioningError::KeyNotFound(_)) => {
                    return Err(VersioningError::KeyNotFound(key.to_string()));
                },
                Err(e) => return Err(e),
            }
        }
        
        // Return the history
        if let Some(history) = histories.get(key) {
            Ok(history.clone())
        } else {
            Err(VersioningError::KeyNotFound(key.to_string()))
        }
    }

    /// Set the current version for a key
    pub async fn set_current_version(
        &self,
        key: &str,
        version_id: &str,
    ) -> VersioningResult<()> {
        let mut histories = self.histories.write().await;
        
        // Get or load the version history
        if !histories.contains_key(key) {
            match self.get_json::<VersionHistory>(&format!("version_history/{}", key)).await {
                Ok(history) => {
                    histories.insert(key.to_string(), history);
                },
                Err(VersioningError::KeyNotFound(_)) => {
                    return Err(VersioningError::KeyNotFound(key.to_string()));
                },
                Err(e) => return Err(e),
            }
        }
        
        // Set the current version
        if let Some(history) = histories.get_mut(key) {
            if history.set_current_version(version_id) {
                // Store updated history
                self.store_json(&format!("version_history/{}", key), history)
                    .await?;
                
                Ok(())
            } else {
                Err(VersioningError::VersionNotFound(version_id.to_string()))
            }
        } else {
            Err(VersioningError::KeyNotFound(key.to_string()))
        }
    }

    /// Delete a specific version
    pub async fn delete_version(
        &self,
        key: &str,
        version_id: &str,
    ) -> VersioningResult<VersionInfo> {
        let mut histories = self.histories.write().await;
        
        // Get or load the version history
        if !histories.contains_key(key) {
            match self.get_json::<VersionHistory>(&format!("version_history/{}", key)).await {
                Ok(history) => {
                    histories.insert(key.to_string(), history);
                },
                Err(VersioningError::KeyNotFound(_)) => {
                    return Err(VersioningError::KeyNotFound(key.to_string()));
                },
                Err(e) => return Err(e),
            }
        }
        
        // Delete the version
        if let Some(history) = histories.get_mut(key) {
            // Make sure we're not deleting the current version
            if history.current_version_id.as_deref() == Some(version_id) {
                return Err(VersioningError::InvalidVersion(
                    "Cannot delete the current version".to_string(),
                ));
            }
            
            // Remove the version
            if let Some(version) = history.versions.remove(version_id) {
                // Update total size
                history.total_size_bytes = history.total_size_bytes.saturating_sub(version.size_bytes);
                
                // Store updated history
                self.store_json(&format!("version_history/{}", key), history)
                    .await?;
                
                // Delete the version data
                let _ = self.storage.delete(&version.storage_key).await;
                
                Ok(version)
            } else {
                Err(VersioningError::VersionNotFound(version_id.to_string()))
            }
        } else {
            Err(VersioningError::KeyNotFound(key.to_string()))
        }
    }

    /// Delete all versions for a key
    pub async fn delete_all_versions(&self, key: &str) -> VersioningResult<()> {
        let mut histories = self.histories.write().await;
        
        // Get or load the version history
        if !histories.contains_key(key) {
            match self.get_json::<VersionHistory>(&format!("version_history/{}", key)).await {
                Ok(history) => {
                    histories.insert(key.to_string(), history);
                },
                Err(VersioningError::KeyNotFound(_)) => {
                    return Err(VersioningError::KeyNotFound(key.to_string()));
                },
                Err(e) => return Err(e),
            }
        }
        
        // Get the history and delete all versions
        if let Some(history) = histories.remove(key) {
            // Delete all version data
            for (_, version) in history.versions {
                let _ = self.storage.delete(&version.storage_key).await;
            }
            
            // Delete the history
            let _ = self.storage.delete(&format!("version_history/{}", key)).await;
            
            Ok(())
        } else {
            Err(VersioningError::KeyNotFound(key.to_string()))
        }
    }

    // Helper function to store JSON data
    async fn store_json<T: Serialize>(&self, key: &str, value: &T) -> VersioningResult<()> {
        let json_data = serde_json::to_vec_pretty(value)
            .map_err(|e| VersioningError::Other(format!("Serialization error: {}", e)))?;
        self.storage.put(key, &json_data).await.map_err(VersioningError::Storage)
    }

    // Helper function to retrieve JSON data
    async fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> VersioningResult<T> {
        match self.storage.get(key).await {
            Ok(data) => {
                serde_json::from_slice(&data)
                    .map_err(|e| VersioningError::Other(format!("Deserialization error: {}", e)))
            },
            Err(StorageError::KeyNotFound(_)) => {
                Err(VersioningError::KeyNotFound(key.to_string()))
            },
            Err(e) => Err(VersioningError::Storage(e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::mock_storage::MockStorage;
    use std::time::{SystemTime, UNIX_EPOCH};

    // Helper to get current timestamp
    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }

    #[tokio::test]
    async fn test_versioning_basic_workflow() {
        // Setup
        let storage = Arc::new(MockStorage::new());
        let versioning = VersioningManager::new(storage.clone(), 5);
        let key = "test-key";
        
        // Create initial version
        let version_id = versioning.generate_version_id();
        let version_storage_key = versioning.create_version_storage_key(key, &version_id);
        
        // Store version data
        storage.put(&version_storage_key, b"Initial version data").await.unwrap();
        
        // Create version info
        let version = VersionInfo {
            version_id: version_id.clone(),
            created_at: current_timestamp(),
            size_bytes: 20, // Length of "Initial version data"
            metadata: HashMap::new(),
            storage_key: version_storage_key,
            content_hash: "fakehash123".to_string(),
            created_by: "test-user".to_string(),
            comment: Some("Initial version".to_string()),
        };
        
        // Initialize versioning
        versioning.init_versioning(key, &version_id, version).await.unwrap();
        
        // Verify history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.version_count(), 1);
        assert_eq!(history.current_version_id, Some(version_id.clone()));
        
        // Add another version
        let version_id2 = versioning.generate_version_id();
        let version_storage_key2 = versioning.create_version_storage_key(key, &version_id2);
        
        // Store version data
        storage.put(&version_storage_key2, b"Updated version data").await.unwrap();
        
        // Create version info
        let version2 = VersionInfo {
            version_id: version_id2.clone(),
            created_at: current_timestamp(),
            size_bytes: 20, // Length of "Updated version data"
            metadata: HashMap::new(),
            storage_key: version_storage_key2,
            content_hash: "fakehash456".to_string(),
            created_by: "test-user".to_string(),
            comment: Some("Second version".to_string()),
        };
        
        // Create version
        versioning.create_version(key, &version_id2, version2).await.unwrap();
        
        // Verify history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.version_count(), 2);
        
        // Set current version
        versioning.set_current_version(key, &version_id2).await.unwrap();
        
        // Verify current version
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.current_version_id, Some(version_id2.clone()));
        
        // Get specific version
        let version = versioning.get_version(key, &version_id).await.unwrap();
        assert_eq!(version.comment, Some("Initial version".to_string()));
        
        // Delete version
        versioning.delete_version(key, &version_id).await.unwrap();
        
        // Verify history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.version_count(), 1);
        
        // Delete all versions
        versioning.delete_all_versions(key).await.unwrap();
        
        // Verify key no longer exists
        let result = versioning.get_version_history(key).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_version_limits() {
        // Setup with a limit of 3 versions
        let storage = Arc::new(MockStorage::new());
        let versioning = VersioningManager::new(storage.clone(), 3);
        let key = "test-key";
        
        // Create 5 versions directly (we'll only keep the most recent 3)
        let mut version_ids = Vec::new();
        
        for i in 1..=5 {
            // Generate a new version
            let vid = versioning.generate_version_id();
            version_ids.push(vid.clone());
            
            let vstorage_key = versioning.create_version_storage_key(key, &vid);
            if let Err(e) = storage.put(&vstorage_key, format!("Version {} data", i).as_bytes()).await {
                panic!("Failed to put version {} data: {:?}", i, e);
            }
            
            // Sleep a tiny bit to ensure different timestamps
            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
            
            let v = VersionInfo {
                version_id: vid.clone(),
                created_at: current_timestamp(),
                size_bytes: 14,
                metadata: HashMap::new(),
                storage_key: vstorage_key,
                content_hash: format!("hash{}", i).to_string(),
                created_by: "test-user".to_string(),
                comment: Some(format!("Version {}", i)),
            };
            
            // For the first version, initialize versioning
            if i == 1 {
                if let Err(e) = versioning.init_versioning(key, &vid, v).await {
                    panic!("Failed to init versioning: {:?}", e);
                }
            } else {
                // For subsequent versions, create a new version
                if let Err(e) = versioning.create_version(key, &vid, v).await {
                    panic!("Failed to create version {}: {:?}", i, e);
                }
                
                // Set as current version
                if let Err(e) = versioning.set_current_version(key, &vid).await {
                    panic!("Failed to set current version to {}: {:?}", i, e);
                }
            }
        }
        
        // Verify we only have 3 versions (the limit) and oldest ones were removed
        match versioning.get_version_history(key).await {
            Ok(history) => {
                assert_eq!(history.version_count(), 3, "Expected 3 versions due to limit, but found {}", history.version_count());
                
                // The first two versions should be gone (indexes 0 and 1)
                for i in 0..2 {
                    let version_id = &version_ids[i];
                    let result = versioning.get_version(key, version_id).await;
                    assert!(matches!(result, Err(VersioningError::VersionNotFound(_))), 
                        "Version {} should have been removed but it still exists", i+1);
                }
                
                // The last three versions should exist (indexes 2, 3, and 4)
                for i in 2..5 {
                    let version_id = &version_ids[i];
                    match versioning.get_version(key, version_id).await {
                        Ok(_) => {}, // Version exists as expected
                        Err(e) => panic!("Version {} should exist but got error: {:?}", i+1, e),
                    }
                }
                
                // Verify the current version is the last one we created
                let current_version = history.get_current_version()
                    .expect("There should be a current version");
                assert_eq!(current_version.version_id, version_ids[4], 
                    "The current version should be the last one created");
            },
            Err(e) => panic!("Failed to get version history: {:?}", e),
        }
        
        // Clean up - ignore errors during cleanup
        let _ = versioning.delete_all_versions(key).await;
    }
} 