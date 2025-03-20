use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;
use async_trait::async_trait;
use thiserror::Error;

use crate::{Storage, StorageResult, StorageError};

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
        let history_json = serde_json::to_vec(&history)?;
        self.storage.put(&history_key, &history_json).await?;
        
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
        
        let history_data = self.storage.get(&history_key).await?;
        let history: VersionHistory = serde_json::from_slice(&history_data)?;
        
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
        let history_json = serde_json::to_vec(&history)?;
        self.storage.put(&history_key, &history_json).await?;
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        Ok(())
    }
    
    /// Get a specific version
    pub async fn get_version(&self, key: &str, version_id: &str) -> VersioningResult<VersionInfo> {
        let history = self.get_version_history(key).await?;
        history.get_version(version_id)
            .cloned()
            .ok_or_else(|| VersioningError::VersionNotFound(version_id.to_string()))
    }
    
    /// Set the current version
    pub async fn set_current_version(&self, key: &str, version_id: &str) -> VersioningResult<()> {
        let mut history = self.get_version_history(key).await?;
        
        if !history.set_current_version(version_id) {
            return Err(VersioningError::VersionNotFound(version_id.to_string()));
        }
        
        // Save the updated history
        let history_key = self.history_key(key);
        let history_json = serde_json::to_vec(&history)?;
        self.storage.put(&history_key, &history_json).await?;
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        Ok(())
    }
    
    /// Delete a specific version
    pub async fn delete_version(&self, key: &str, version_id: &str) -> VersioningResult<()> {
        let mut history = self.get_version_history(key).await?;
        
        // Find the version to delete
        let version_index = history.versions.iter()
            .position(|v| v.version_id == version_id)
            .ok_or_else(|| VersioningError::VersionNotFound(version_id.to_string()))?;
        
        // Remove the version
        let removed_version = history.versions.remove(version_index);
        history.total_size_bytes = history.total_size_bytes.saturating_sub(removed_version.size_bytes);
        
        // Update current version if needed
        if history.current_version_id.as_ref() == Some(version_id) {
            history.current_version_id = history.latest_version()
                .map(|v| v.version_id.clone());
        }
        
        // Delete the version data
        let version_key = self.create_version_storage_key(key, version_id);
        if let Err(e) = self.storage.delete(&version_key).await {
            // Log the error but continue with history update
            tracing::error!("Failed to delete version data: {}", e);
        }
        
        // Save the updated history
        let history_key = self.history_key(key);
        let history_json = serde_json::to_vec(&history)?;
        self.storage.put(&history_key, &history_json).await?;
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.insert(key.to_string(), history);
        
        Ok(())
    }
    
    /// Delete all versions for a key
    pub async fn delete_all_versions(&self, key: &str) -> VersioningResult<()> {
        let history = self.get_version_history(key).await?;
        
        // Delete all version data
        for version in &history.versions {
            let version_key = self.create_version_storage_key(key, &version.version_id);
            if let Err(e) = self.storage.delete(&version_key).await {
                // Log the error but continue with other deletions
                tracing::error!("Failed to delete version data: {}", e);
            }
        }
        
        // Delete the history
        let history_key = self.history_key(key);
        if let Err(e) = self.storage.delete(&history_key).await {
            // Log the error but continue with cache update
            tracing::error!("Failed to delete history: {}", e);
        }
        
        // Update cache
        let mut histories = self.histories.write().await;
        histories.remove(key);
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_versioning_basic_workflow() {
        let storage = Arc::new(MemoryStorage::new());
        let versioning = VersioningManager::new(Arc::clone(&storage), 3);
        
        let key = "test-key";
        let version_id = versioning.generate_version_id();
        
        // Create initial version
        let version_info = VersionInfo {
            version_id: version_id.clone(),
            created_at: 1000,
            size_bytes: 100,
            metadata: HashMap::new(),
            storage_key: versioning.create_version_storage_key(key, &version_id),
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
        
        // Create new version
        let version_id2 = versioning.generate_version_id();
        let version_info2 = VersionInfo {
            version_id: version_id2.clone(),
            created_at: 1001,
            size_bytes: 200,
            metadata: HashMap::new(),
            storage_key: versioning.create_version_storage_key(key, &version_id2),
            content_hash: "hash2".to_string(),
            created_by: "test-user".to_string(),
            comment: Some("Second version".to_string()),
        };
        
        versioning.create_version(key, &version_id2, version_info2).await.unwrap();
        
        // Get updated history
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 2);
        assert_eq!(history.current_version_id, Some(version_id2.clone()));
        
        // Set current version
        versioning.set_current_version(key, &version_id).await.unwrap();
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.current_version_id, Some(version_id.clone()));
        
        // Delete version
        versioning.delete_version(key, &version_id2).await.unwrap();
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 1);
        assert_eq!(history.current_version_id, Some(version_id.clone()));
    }
    
    #[tokio::test]
    async fn test_version_limits() {
        let storage = Arc::new(MemoryStorage::new());
        let versioning = VersioningManager::new(Arc::clone(&storage), 2);
        
        let key = "test-key";
        
        // Create three versions
        for i in 0..3 {
            let version_id = versioning.generate_version_id();
            let version_info = VersionInfo {
                version_id: version_id.clone(),
                created_at: 1000 + i,
                size_bytes: 100,
                metadata: HashMap::new(),
                storage_key: versioning.create_version_storage_key(key, &version_id),
                content_hash: format!("hash{}", i + 1),
                created_by: "test-user".to_string(),
                comment: Some(format!("Version {}", i + 1)),
            };
            
            if i == 0 {
                versioning.init_versioning(key, None, Some(version_info)).await.unwrap();
            } else {
                versioning.create_version(key, &version_id, version_info).await.unwrap();
            }
        }
        
        // Check that only the latest two versions are kept
        let history = versioning.get_version_history(key).await.unwrap();
        assert_eq!(history.versions.len(), 2);
        assert_eq!(history.current_version_id, Some(versioning.generate_version_id()));
    }
} 