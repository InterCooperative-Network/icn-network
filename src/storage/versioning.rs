use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::crypto::StorageEncryptionService;

// Version information for a data object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    // Version identifier
    pub version_id: String,
    // The actual key where this version's data is stored
    pub storage_key: String,
    // When this version was created
    pub created_at: u64,
    // Size of this version in bytes
    pub size_bytes: u64,
    // Content hash for integrity verification
    pub content_hash: String,
    // Who created this version (node ID)
    pub created_by: String,
    // Optional comment about this version
    pub comment: Option<String>,
    // Version metadata for application-specific use
    pub metadata: HashMap<String, String>,
}

// Version history for a data object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    // The original data key
    pub base_key: String,
    // Current active version ID
    pub current_version_id: String,
    // All versions, mapped by version ID
    pub versions: HashMap<String, VersionInfo>,
    // Ordered list of version IDs from newest to oldest
    pub version_timeline: Vec<String>,
    // Whether to keep all versions or limit them
    pub retain_all_versions: bool,
    // Maximum number of versions to keep if not retaining all
    pub max_versions: u32,
    // Total storage used by all versions in bytes
    pub total_size_bytes: u64,
}

impl VersionHistory {
    // Create a new version history for a key
    pub fn new(base_key: &str, initial_version_id: &str, max_versions: u32) -> Self {
        Self {
            base_key: base_key.to_string(),
            current_version_id: initial_version_id.to_string(),
            versions: HashMap::new(),
            version_timeline: vec![initial_version_id.to_string()],
            retain_all_versions: false,
            max_versions,
            total_size_bytes: 0,
        }
    }
    
    // Add a new version to the history
    pub fn add_version(&mut self, version_info: VersionInfo) {
        // Add size to total
        self.total_size_bytes += version_info.size_bytes;
        
        // Add to versions map
        let version_id = version_info.version_id.clone();
        self.versions.insert(version_id.clone(), version_info);
        
        // Update timeline (add to front as it's newest to oldest)
        self.version_timeline.insert(0, version_id.clone());
        
        // Set as current version
        self.current_version_id = version_id;
        
        // Enforce version limit if needed
        if !self.retain_all_versions && self.version_timeline.len() > self.max_versions as usize {
            // Remove oldest versions beyond the limit
            while self.version_timeline.len() > self.max_versions as usize {
                if let Some(oldest_id) = self.version_timeline.pop() {
                    if let Some(removed) = self.versions.remove(&oldest_id) {
                        // Update total size
                        self.total_size_bytes -= removed.size_bytes;
                    }
                }
            }
        }
    }
    
    // Get a specific version by ID
    pub fn get_version(&self, version_id: &str) -> Option<&VersionInfo> {
        self.versions.get(version_id)
    }
    
    // Get the current active version
    pub fn get_current_version(&self) -> Option<&VersionInfo> {
        self.versions.get(&self.current_version_id)
    }
    
    // Set a different version as the current active version
    pub fn set_current_version(&mut self, version_id: &str) -> Result<(), VersioningError> {
        if !self.versions.contains_key(version_id) {
            return Err(VersioningError::VersionNotFound(format!(
                "Version {} not found for key {}", 
                version_id, self.base_key
            )));
        }
        
        self.current_version_id = version_id.to_string();
        Ok(())
    }
    
    // Get all versions in timeline order (newest to oldest)
    pub fn get_all_versions(&self) -> Vec<&VersionInfo> {
        self.version_timeline.iter()
            .filter_map(|id| self.versions.get(id))
            .collect()
    }
}

// Versioning error types
#[derive(Debug)]
pub enum VersioningError {
    KeyNotFound(String),
    VersionNotFound(String),
    StorageError(String),
    SerializationError(String),
    AccessDenied(String),
}

impl fmt::Display for VersioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KeyNotFound(msg) => write!(f, "Key not found: {}", msg),
            Self::VersionNotFound(msg) => write!(f, "Version not found: {}", msg),
            Self::StorageError(msg) => write!(f, "Storage error: {}", msg),
            Self::SerializationError(msg) => write!(f, "Serialization error: {}", msg),
            Self::AccessDenied(msg) => write!(f, "Access denied: {}", msg),
        }
    }
}

impl Error for VersioningError {}

// Versioning manager for distributed storage
pub struct VersioningManager {
    // Version histories for data objects, keyed by base key
    version_histories: RwLock<HashMap<String, VersionHistory>>,
    // Encryption service for version data
    encryption_service: Arc<StorageEncryptionService>,
}

impl VersioningManager {
    // Create a new versioning manager
    pub fn new(encryption_service: Arc<StorageEncryptionService>) -> Self {
        Self {
            version_histories: RwLock::new(HashMap::new()),
            encryption_service,
        }
    }
    
    // Initialize versioning for a data key
    pub async fn init_versioning(
        &self,
        base_key: &str,
        initial_version_id: &str,
        initial_version_info: VersionInfo,
        max_versions: u32,
    ) -> Result<(), VersioningError> {
        let mut histories = self.version_histories.write().await;
        
        // Check if already versioned
        if histories.contains_key(base_key) {
            return Ok(());  // Already initialized
        }
        
        // Create new version history
        let mut history = VersionHistory::new(base_key, initial_version_id, max_versions);
        
        // Add initial version
        history.add_version(initial_version_info);
        
        // Store history
        histories.insert(base_key.to_string(), history);
        
        Ok(())
    }
    
    // Create a new version for a data key
    pub async fn create_version(
        &self,
        base_key: &str,
        version_id: &str,
        version_info: VersionInfo,
    ) -> Result<(), VersioningError> {
        let mut histories = self.version_histories.write().await;
        
        // Get history for this key
        let history = histories.get_mut(base_key).ok_or_else(|| {
            VersioningError::KeyNotFound(format!("No version history for key: {}", base_key))
        })?;
        
        // Add the new version
        history.add_version(version_info);
        
        Ok(())
    }
    
    // Get the current version info for a key
    pub async fn get_current_version(&self, base_key: &str) -> Result<VersionInfo, VersioningError> {
        let histories = self.version_histories.read().await;
        
        // Get history for this key
        let history = histories.get(base_key).ok_or_else(|| {
            VersioningError::KeyNotFound(format!("No version history for key: {}", base_key))
        })?;
        
        // Get current version
        let version = history.get_current_version().ok_or_else(|| {
            VersioningError::VersionNotFound(format!("No current version for key: {}", base_key))
        })?;
        
        Ok(version.clone())
    }
    
    // Set the current version for a key
    pub async fn set_current_version(&self, base_key: &str, version_id: &str) -> Result<(), VersioningError> {
        let mut histories = self.version_histories.write().await;
        
        // Get history for this key
        let history = histories.get_mut(base_key).ok_or_else(|| {
            VersioningError::KeyNotFound(format!("No version history for key: {}", base_key))
        })?;
        
        // Set current version
        history.set_current_version(version_id)?;
        
        Ok(())
    }
    
    // Get a specific version for a key
    pub async fn get_version(&self, base_key: &str, version_id: &str) -> Result<VersionInfo, VersioningError> {
        let histories = self.version_histories.read().await;
        
        // Get history for this key
        let history = histories.get(base_key).ok_or_else(|| {
            VersioningError::KeyNotFound(format!("No version history for key: {}", base_key))
        })?;
        
        // Get specific version
        let version = history.get_version(version_id).ok_or_else(|| {
            VersioningError::VersionNotFound(format!(
                "Version {} not found for key {}", 
                version_id, base_key
            ))
        })?;
        
        Ok(version.clone())
    }
    
    // Get version history for a key
    pub async fn get_version_history(&self, base_key: &str) -> Result<VersionHistory, VersioningError> {
        let histories = self.version_histories.read().await;
        
        // Get history for this key
        let history = histories.get(base_key).ok_or_else(|| {
            VersioningError::KeyNotFound(format!("No version history for key: {}", base_key))
        })?;
        
        Ok(history.clone())
    }
    
    // Delete a specific version
    pub async fn delete_version(&self, base_key: &str, version_id: &str) -> Result<(), VersioningError> {
        let mut histories = self.version_histories.write().await;
        
        // Get history for this key
        let history = histories.get_mut(base_key).ok_or_else(|| {
            VersioningError::KeyNotFound(format!("No version history for key: {}", base_key))
        })?;
        
        // Ensure this isn't the only version
        if history.versions.len() <= 1 {
            return Err(VersioningError::StorageError(
                format!("Cannot delete the only version for key: {}", base_key)
            ));
        }
        
        // Ensure this isn't the current version
        if history.current_version_id == version_id {
            return Err(VersioningError::StorageError(
                format!("Cannot delete the current version. Set a different version as current first.")
            ));
        }
        
        // Remove from timeline
        history.version_timeline.retain(|id| id != version_id);
        
        // Remove from versions map and update total size
        if let Some(removed) = history.versions.remove(version_id) {
            history.total_size_bytes -= removed.size_bytes;
        }
        
        Ok(())
    }
    
    // Generate a unique version ID
    pub fn generate_version_id(&self) -> String {
        use rand::{Rng, thread_rng};
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let random: u32 = thread_rng().gen();
        format!("v-{}-{:08x}", timestamp, random)
    }
    
    // Utility to create a storage key for a versioned object
    pub fn create_version_storage_key(&self, base_key: &str, version_id: &str) -> String {
        format!("{}.versions/{}", base_key, version_id)
    }
} 