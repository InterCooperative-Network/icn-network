use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{Storage, StorageResult, StorageError};

/// Quota-related errors
#[derive(Debug, Error)]
pub enum QuotaError {
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    
    #[error("Invalid quota: {0}")]
    InvalidQuota(String),
    
    #[error("Quota not found: {0}")]
    QuotaNotFound(String),
    
    #[error("Other quota error: {0}")]
    Other(String),
}

pub type QuotaResult<T> = Result<T, QuotaError>;

/// Quota configuration for a user or group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaConfig {
    pub max_size_bytes: u64,
    pub max_objects: u64,
    pub max_versions_per_object: u32,
    pub max_bandwidth_bytes_per_second: u64,
}

impl QuotaConfig {
    /// Create a new quota configuration
    pub fn new(
        max_size_bytes: u64,
        max_objects: u64,
        max_versions_per_object: u32,
        max_bandwidth_bytes_per_second: u64,
    ) -> Self {
        Self {
            max_size_bytes,
            max_objects,
            max_versions_per_object,
            max_bandwidth_bytes_per_second,
        }
    }
    
    /// Create a default quota configuration
    pub fn default() -> Self {
        Self {
            max_size_bytes: 1024 * 1024 * 1024, // 1GB
            max_objects: 1000,
            max_versions_per_object: 10,
            max_bandwidth_bytes_per_second: 1024 * 1024, // 1MB/s
        }
    }
}

/// Current usage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageStats {
    pub total_size_bytes: u64,
    pub total_objects: u64,
    pub total_versions: u64,
    pub current_bandwidth_bytes_per_second: u64,
}

impl UsageStats {
    /// Create new usage statistics
    pub fn new() -> Self {
        Self {
            total_size_bytes: 0,
            total_objects: 0,
            total_versions: 0,
            current_bandwidth_bytes_per_second: 0,
        }
    }
    
    /// Update bandwidth usage
    pub fn update_bandwidth(&mut self, bytes_per_second: u64) {
        self.current_bandwidth_bytes_per_second = bytes_per_second;
    }
    
    /// Add object size
    pub fn add_size(&mut self, size_bytes: u64) {
        self.total_size_bytes += size_bytes;
    }
    
    /// Remove object size
    pub fn remove_size(&mut self, size_bytes: u64) {
        self.total_size_bytes = self.total_size_bytes.saturating_sub(size_bytes);
    }
    
    /// Increment object count
    pub fn increment_objects(&mut self) {
        self.total_objects += 1;
    }
    
    /// Decrement object count
    pub fn decrement_objects(&mut self) {
        if self.total_objects > 0 {
            self.total_objects -= 1;
        }
    }
    
    /// Add version count
    pub fn add_versions(&mut self, count: u64) {
        self.total_versions += count;
    }
    
    /// Remove version count
    pub fn remove_versions(&mut self, count: u64) {
        self.total_versions = self.total_versions.saturating_sub(count);
    }
}

/// Quota manager for storage
pub struct QuotaManager {
    storage: Arc<dyn Storage>,
    quotas: RwLock<HashMap<String, QuotaConfig>>,
    usage: RwLock<HashMap<String, UsageStats>>,
    quota_prefix: String,
    usage_prefix: String,
}

impl QuotaManager {
    /// Create a new quota manager
    pub fn new(storage: Arc<dyn Storage>) -> Self {
        Self {
            storage,
            quotas: RwLock::new(HashMap::new()),
            usage: RwLock::new(HashMap::new()),
            quota_prefix: "_quotas/".to_string(),
            usage_prefix: "_usage/".to_string(),
        }
    }
    
    /// Get the key for storing quota configuration
    fn quota_key(&self, user_id: &str) -> String {
        format!("{}{}", self.quota_prefix, user_id)
    }
    
    /// Get the key for storing usage statistics
    fn usage_key(&self, user_id: &str) -> String {
        format!("{}{}", self.usage_prefix, user_id)
    }
    
    /// Set quota configuration for a user
    pub async fn set_quota(&self, user_id: &str, quota: QuotaConfig) -> QuotaResult<()> {
        let quota_key = self.quota_key(user_id);
        let quota_json = serde_json::to_vec(&quota)?;
        
        // Store quota configuration
        self.storage.put(&quota_key, &quota_json).await?;
        
        // Update cache
        let mut quotas = self.quotas.write().await;
        quotas.insert(user_id.to_string(), quota);
        
        Ok(())
    }
    
    /// Get quota configuration for a user
    pub async fn get_quota(&self, user_id: &str) -> QuotaResult<QuotaConfig> {
        // Check cache first
        {
            let quotas = self.quotas.read().await;
            if let Some(quota) = quotas.get(user_id) {
                return Ok(quota.clone());
            }
        }
        
        // Try to load from storage
        let quota_key = self.quota_key(user_id);
        if !self.storage.exists(&quota_key).await? {
            return Ok(QuotaConfig::default());
        }
        
        let quota_data = self.storage.get(&quota_key).await?;
        let quota: QuotaConfig = serde_json::from_slice(&quota_data)?;
        
        // Cache the quota
        let mut quotas = self.quotas.write().await;
        quotas.insert(user_id.to_string(), quota.clone());
        
        Ok(quota)
    }
    
    /// Get usage statistics for a user
    pub async fn get_usage(&self, user_id: &str) -> QuotaResult<UsageStats> {
        // Check cache first
        {
            let usage = self.usage.read().await;
            if let Some(stats) = usage.get(user_id) {
                return Ok(stats.clone());
            }
        }
        
        // Try to load from storage
        let usage_key = self.usage_key(user_id);
        if !self.storage.exists(&usage_key).await? {
            return Ok(UsageStats::new());
        }
        
        let usage_data = self.storage.get(&usage_key).await?;
        let stats: UsageStats = serde_json::from_slice(&usage_data)?;
        
        // Cache the stats
        let mut usage = self.usage.write().await;
        usage.insert(user_id.to_string(), stats.clone());
        
        Ok(stats)
    }
    
    /// Update usage statistics for a user
    pub async fn update_usage(&self, user_id: &str, stats: UsageStats) -> QuotaResult<()> {
        let usage_key = self.usage_key(user_id);
        let usage_json = serde_json::to_vec(&stats)?;
        
        // Store usage statistics
        self.storage.put(&usage_key, &usage_json).await?;
        
        // Update cache
        let mut usage = self.usage.write().await;
        usage.insert(user_id.to_string(), stats);
        
        Ok(())
    }
    
    /// Check if a user has exceeded their quota
    pub async fn check_quota(&self, user_id: &str, size_bytes: u64) -> QuotaResult<()> {
        let quota = self.get_quota(user_id).await?;
        let usage = self.get_usage(user_id).await?;
        
        // Check size quota
        if usage.total_size_bytes + size_bytes > quota.max_size_bytes {
            return Err(QuotaError::QuotaExceeded(format!(
                "Storage quota exceeded: {} bytes used, {} bytes limit",
                usage.total_size_bytes, quota.max_size_bytes
            )));
        }
        
        // Check object count quota
        if usage.total_objects >= quota.max_objects {
            return Err(QuotaError::QuotaExceeded(format!(
                "Object count quota exceeded: {} objects, {} limit",
                usage.total_objects, quota.max_objects
            )));
        }
        
        // Check bandwidth quota
        if usage.current_bandwidth_bytes_per_second > quota.max_bandwidth_bytes_per_second {
            return Err(QuotaError::QuotaExceeded(format!(
                "Bandwidth quota exceeded: {} bytes/s, {} bytes/s limit",
                usage.current_bandwidth_bytes_per_second,
                quota.max_bandwidth_bytes_per_second
            )));
        }
        
        Ok(())
    }
    
    /// Record object creation
    pub async fn record_object_creation(&self, user_id: &str, size_bytes: u64) -> QuotaResult<()> {
        let mut usage = self.get_usage(user_id).await?;
        usage.add_size(size_bytes);
        usage.increment_objects();
        self.update_usage(user_id, usage).await
    }
    
    /// Record object deletion
    pub async fn record_object_deletion(&self, user_id: &str, size_bytes: u64) -> QuotaResult<()> {
        let mut usage = self.get_usage(user_id).await?;
        usage.remove_size(size_bytes);
        usage.decrement_objects();
        self.update_usage(user_id, usage).await
    }
    
    /// Record version creation
    pub async fn record_version_creation(&self, user_id: &str, count: u64) -> QuotaResult<()> {
        let mut usage = self.get_usage(user_id).await?;
        usage.add_versions(count);
        self.update_usage(user_id, usage).await
    }
    
    /// Record version deletion
    pub async fn record_version_deletion(&self, user_id: &str, count: u64) -> QuotaResult<()> {
        let mut usage = self.get_usage(user_id).await?;
        usage.remove_versions(count);
        self.update_usage(user_id, usage).await
    }
    
    /// Update bandwidth usage
    pub async fn update_bandwidth(&self, user_id: &str, bytes_per_second: u64) -> QuotaResult<()> {
        let mut usage = self.get_usage(user_id).await?;
        usage.update_bandwidth(bytes_per_second);
        self.update_usage(user_id, usage).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_quota_management() {
        let storage = Arc::new(MemoryStorage::new());
        let quota_manager = QuotaManager::new(Arc::clone(&storage));
        
        let user_id = "test-user";
        
        // Set custom quota
        let quota = QuotaConfig::new(1000, 10, 5, 100);
        quota_manager.set_quota(user_id, quota.clone()).await.unwrap();
        
        // Get quota
        let retrieved_quota = quota_manager.get_quota(user_id).await.unwrap();
        assert_eq!(retrieved_quota.max_size_bytes, 1000);
        assert_eq!(retrieved_quota.max_objects, 10);
        assert_eq!(retrieved_quota.max_versions_per_object, 5);
        assert_eq!(retrieved_quota.max_bandwidth_bytes_per_second, 100);
        
        // Check initial usage
        let usage = quota_manager.get_usage(user_id).await.unwrap();
        assert_eq!(usage.total_size_bytes, 0);
        assert_eq!(usage.total_objects, 0);
        assert_eq!(usage.total_versions, 0);
        assert_eq!(usage.current_bandwidth_bytes_per_second, 0);
        
        // Record object creation
        quota_manager.record_object_creation(user_id, 500).await.unwrap();
        let usage = quota_manager.get_usage(user_id).await.unwrap();
        assert_eq!(usage.total_size_bytes, 500);
        assert_eq!(usage.total_objects, 1);
        
        // Check quota limits
        assert!(quota_manager.check_quota(user_id, 600).await.is_err()); // Would exceed size quota
        assert!(quota_manager.check_quota(user_id, 100).await.is_ok());
        
        // Record object deletion
        quota_manager.record_object_deletion(user_id, 500).await.unwrap();
        let usage = quota_manager.get_usage(user_id).await.unwrap();
        assert_eq!(usage.total_size_bytes, 0);
        assert_eq!(usage.total_objects, 0);
        
        // Update bandwidth
        quota_manager.update_bandwidth(user_id, 50).await.unwrap();
        let usage = quota_manager.get_usage(user_id).await.unwrap();
        assert_eq!(usage.current_bandwidth_bytes_per_second, 50);
        
        // Check bandwidth quota
        assert!(quota_manager.update_bandwidth(user_id, 150).await.is_ok()); // Cache update succeeds
        assert!(quota_manager.check_quota(user_id, 0).await.is_err()); // But quota check fails
    }
} 