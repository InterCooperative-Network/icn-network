use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use thiserror::Error;
use super::{Storage, StorageResult, StorageError, JsonStorage};

use crate::crypto::StorageEncryptionService;

/// Version information for a stored object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version_id: String,
    pub created_at: u64,
    pub size_bytes: u64,
    pub metadata: HashMap<String, String>,
    pub storage_key: String,
    pub content_hash: String,
    pub created_by: String,
    pub comment: Option<String>,
}

/// Version history for a key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    pub key: String,
    pub versions: Vec<VersionInfo>,
    pub max_versions: u32,
    pub current_version_id: Option<String>,
    pub total_size_bytes: u64,
}

impl VersionHistory {
    /// Create a new version history
    pub fn new(key: &str, max_versions: u32) -> Self {
        VersionHistory {
            key: key.to_string(),
            versions: Vec::new(),
            max_versions,
            current_version_id: None,
            total_size_bytes: 0,
        }
    }
    
    /// Add a version to the history
    pub fn add_version(&mut self, version: VersionInfo) {
        let version_size = version.size_bytes;
        self.total_size_bytes += version_size;
        
        // Set as current version if it's the first one
        if self.versions.is_empty() {
            self.current_version_id = Some(version.version_id.clone());
        }
        
        self.versions.push(version);
        
        // Trim history if we have too many versions
        while self.versions.len() > self.max_versions as usize {
            if let Some(removed_version) = self.versions.remove(0) {
                self.total_size_bytes = self.total_size_bytes.saturating_sub(removed_version.size_bytes);
            }
        }
    }
    
    /// Get a specific version
    pub fn get_version(&self, version_id: &str) -> Option<&VersionInfo> {
        self.versions.iter().find(|v| v.version_id == version_id)
    }
    
    /// Set the current version
    pub fn set_current_version(&mut self, version_id: &str) -> bool {
        if self.get_version(version_id).is_some() {
            self.current_version_id = Some(version_id.to_string());
            true
        } else {
            false
        }
    }
    
    /// Get the current version
    pub fn current_version(&self) -> Option<&VersionInfo> {
        if let Some(ref id) = self.current_version_id {
            self.get_version(id)
        } else {
            None
        }
    }
    
    /// Get the latest version (most recently added)
    pub fn latest_version(&self) -> Option<&VersionInfo> {
        self.versions.last()
    }
}

/// Versioning-related errors
#[derive(Debug, Error)]
pub enum VersioningError {
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    #[error("Invalid version: {0}")]
    InvalidVersion(String),
    
    #[error("Version conflict: {0}")]
    VersionConflict(String),
    
    #[error("Other versioning error: {0}")]
    Other(String),
}

pub type VersioningResult<T> = Result<T, VersioningError>;

/// Versioning manager for storage objects
pub struct VersioningManager {
    storage: Arc<dyn Storage>,
    histories: RwLock<HashMap<String, VersionHistory>>,
    version_prefix: String,
    history_prefix: String,
    max_versions_default: u32,
}

impl VersioningManager {
    /// Create a new versioning manager
    pub fn new(storage: Arc<dyn Storage>, max_versions_default: u32) -> Self {
        VersioningManager {
            storage,
            histories: RwLock::new(HashMap::new()),
            version_prefix: "_versions/".to_string(),
            history_prefix: "_histories/".to_string(),
            max_versions_default,
        }
    }
    
    /// Generate a unique version ID
    pub fn generate_version_id(&self) -> String {
        use rand::{Rng, thread_rng};
        let mut rng = thread_rng();
        let rand_part: u64 = rng.gen();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        format!("v-{}-{:x}", timestamp, rand_part)
    }
    
    /// Create a storage key for a version
    pub fn create_version_storage_key(&self, key: &str, version_id: &str) -> String {
        format!("{}{}/{}", self.version_prefix, key, version_id)
    }
    
    /// Get the key for storing version history
    fn history_key(&self, key: &str) -> String {
        format!("{}{}", self.history_prefix, key)
    }
    
    /// Initialize versioning for a key
    pub async fn init_versioning(
        &self,
        key: &str,
        max_versions: Option<u32>,
        first_version: Option<VersionInfo>,
    ) -> VersioningResult<()> {
        let history_key = self.history_key(key);
        
        // Check if versioning is already initialized
        if self.storage.exists(&history_key).await? {
            return Ok(());
        }
        
        let max_versions = max_versions.unwrap_or(self.max_versions_default);
        let mut history = VersionHistory::new(key, max_versions);
        
        if let Some(version) = first_version {
            history.add_version(version);
        }
        
        // Store the history
        self.storage.put_json(&history_key, &history).await?;
        
        // Cache the history
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        Ok(())
    }
    
    /// Get the version history for a key
    pub async fn get_version_history(&self, key: &str) -> VersioningResult<VersionHistory> {
        // Check cache first
        {
            let histories = self.histories.read().await;
            if let Some(history) = histories.get(key) {
                return Ok(history.clone());
            }
        }
        
        // Try to load from storage
        let history_key = self.history_key(key);
        if !self.storage.exists(&history_key).await? {
            return Err(VersioningError::KeyNotFound(key.to_string()));
        }
        
        let history: VersionHistory = self.storage.get_json(&history_key).await?;
        
        // Cache the history
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history.clone());
        
        Ok(history)
    }
    
    /// Create a new version
    pub async fn create_version(
        &self,
        key: &str,
        version_id: &str,
        version_info: VersionInfo,
    ) -> VersioningResult<()> {
        let mut history = self.get_version_history(key).await
            .unwrap_or_else(|_| VersionHistory::new(key, self.max_versions_default));
        
        // Check if version ID already exists
        if history.get_version(version_id).is_some() {
            return Err(VersioningError::VersionConflict(format!(
                "Version {} already exists for key {}", version_id, key
            )));
        }
        
        // Add the version
        history.add_version(version_info);
        
        // Save the updated history
        let history_key = self.history_key(key);
        self.storage.put_json(&history_key, &history).await?;
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        Ok(())
    }
    
    /// Get a specific version
    pub async fn get_version(&self, key: &str, version_id: &str) -> VersioningResult<VersionInfo> {
        let history = self.get_version_history(key).await?;
        
        let version = history.get_version(version_id)
            .ok_or_else(|| VersioningError::VersionNotFound(format!(
                "Version {} not found for key {}", version_id, key
            )))?;
        
        Ok(version.clone())
    }
    
    /// Set the current version
    pub async fn set_current_version(&self, key: &str, version_id: &str) -> VersioningResult<()> {
        let mut history = self.get_version_history(key).await?;
        
        if !history.set_current_version(version_id) {
            return Err(VersioningError::VersionNotFound(format!(
                "Version {} not found for key {}", version_id, key
            )));
        }
        
        // Save the updated history
        let history_key = self.history_key(key);
        self.storage.put_json(&history_key, &history).await?;
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        Ok(())
    }
    
    /// Delete a version
    pub async fn delete_version(&self, key: &str, version_id: &str) -> VersioningResult<()> {
        let mut history = self.get_version_history(key).await?;
        
        // Find the index of the version
        let index = history.versions.iter().position(|v| v.version_id == version_id)
            .ok_or_else(|| VersioningError::VersionNotFound(format!(
                "Version {} not found for key {}", version_id, key
            )))?;
        
        // Check if this is the current version
        if let Some(current_id) = &history.current_version_id {
            if current_id == version_id {
                // Set current to the latest version that's not being deleted
                if let Some(latest) = history.versions.iter()
                    .filter(|v| v.version_id != version_id)
                    .last() {
                    history.current_version_id = Some(latest.version_id.clone());
                } else {
                    history.current_version_id = None;
                }
            }
        }
        
        // Remove from history
        let removed_version = history.versions.remove(index);
        history.total_size_bytes = history.total_size_bytes.saturating_sub(removed_version.size_bytes);
        
        // Save the updated history
        let history_key = self.history_key(key);
        self.storage.put_json(&history_key, &history).await?;
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        // Delete the version data
        let version_key = self.create_version_storage_key(key, version_id);
        self.storage.delete(&version_key).await?;
        
        Ok(())
    }
    
    /// Delete all versions for a key
    pub async fn delete_all_versions(&self, key: &str) -> VersioningResult<()> {
        let history = self.get_version_history(key).await?;
        
        // Delete all version data
        for version in &history.versions {
            let version_key = self.create_version_storage_key(key, &version.version_id);
            self.storage.delete(&version_key).await?;
        }
        
        // Delete the history
        let history_key = self.history_key(key);
        self.storage.delete(&history_key).await?;
        
        // Remove from cache
        let mut histories = self.histories.write().await;
        histories.remove(key);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::memory_storage::MemoryStorage;
    
    #[tokio::test]
    async fn test_versioning_basic_workflow() {
        let storage = Arc::new(MemoryStorage::new());
        let versioning = VersioningManager::new(Arc::clone(&storage), 5);
        
        // Create a version
        let key = "test-key";
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
        versioning.init_versioning(key, None, Some(version_info.clone())).await.unwrap();
        
        // Get history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.current_version_id, Some(version_id.clone()));
        
        // Create a second version
        let version_id2 = versioning.generate_version_id();
        let version_key2 = versioning.create_version_storage_key(key, &version_id2);
        
        // Store version data
        let data2 = b"Version 2 data";
        storage.put(&version_key2, data2).await.unwrap();
        
        // Create version info
        let version_info2 = VersionInfo {
            version_id: version_id2.clone(),
            created_at: 2000,
            size_bytes: data2.len() as u64,
            metadata: HashMap::new(),
            storage_key: version_key2.clone(),
            content_hash: "hash2".to_string(),
            created_by: "test-user".to_string(),
            comment: Some("Second version".to_string()),
        };
        
        // Add the version
        versioning.create_version(key, &version_id2, version_info2.clone()).await.unwrap();
        
        // Get history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 2);
        assert_eq!(history.current_version_id, Some(version_id.clone()));
        
        // Set current version
        versioning.set_current_version(key, &version_id2).await.unwrap();
        
        // Get history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.current_version_id, Some(version_id2.clone()));
        
        // Get version
        let version = versioning.get_version(key, &version_id2).await.unwrap();
        assert_eq!(version.version_id, version_id2);
        assert_eq!(version.comment, Some("Second version".to_string()));
        
        // Delete a version
        versioning.delete_version(key, &version_id).await.unwrap();
        
        // Get history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.current_version_id, Some(version_id2.clone()));
        
        // Delete all versions
        versioning.delete_all_versions(key).await.unwrap();
        
        // Check history is gone
        let result = versioning.get_version_history(key).await;
        assert!(result.is_err());
    }
    
    #[tokio::test]
    async fn test_version_limits() {
        let storage = Arc::new(MemoryStorage::new());
        // Set max versions to 3
        let versioning = VersioningManager::new(Arc::clone(&storage), 3);
        
        let key = "test-key-limits";
        
        // Initialize empty versioning
        versioning.init_versioning(key, None, None).await.unwrap();
        
        // Add 5 versions - should keep only the 3 newest
        for i in 1..=5 {
            let version_id = format!("v{}", i);
            let version_key = versioning.create_version_storage_key(key, &version_id);
            
            // Store mock data
            let data = format!("Data for version {}", i).into_bytes();
            storage.put(&version_key, &data).await.unwrap();
            
            // Create version info
            let version_info = VersionInfo {
                version_id: version_id.clone(),
                created_at: (i * 1000) as u64,
                size_bytes: data.len() as u64,
                metadata: HashMap::new(),
                storage_key: version_key,
                content_hash: format!("hash{}", i),
                created_by: "test-user".to_string(),
                comment: Some(format!("Version {}", i)),
            };
            
            // Add the version
            versioning.create_version(key, &version_id, version_info).await.unwrap();
        }
        
        // Get history - should have only 3 versions
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 3);
        
        // Verify we have versions 3, 4, and 5 (not 1 and 2)
        let version_ids: Vec<_> = history.versions.iter().map(|v| v.version_id.clone()).collect();
        assert!(version_ids.contains(&"v3".to_string()));
        assert!(version_ids.contains(&"v4".to_string()));
        assert!(version_ids.contains(&"v5".to_string()));
        assert!(!version_ids.contains(&"v1".to_string()));
        assert!(!version_ids.contains(&"v2".to_string()));
        
        // Verify the total size is correct (sum of sizes of versions 3, 4, and 5)
        let expected_size = history.versions.iter().fold(0, |acc, v| acc + v.size_bytes);
        assert_eq!(history.total_size_bytes, expected_size);
    }
} 