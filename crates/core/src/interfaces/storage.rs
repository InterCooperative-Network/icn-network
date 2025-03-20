use async_trait::async_trait;
use std::error::Error;
use serde::{Serialize, de::DeserializeOwned};

/// Storage operation types for metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageOperation {
    Read,
    Write,
    Delete,
    List,
}

/// Storage options for configuring operations
#[derive(Debug, Clone, Default)]
pub struct StorageOptions {
    pub namespace: Option<String>,
    pub ttl: Option<u64>,
    pub encryption: bool,
}

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

/// Provider interface for storage-related operations
#[async_trait]
pub trait StorageProvider: Send + Sync {
    /// Store a serializable value with the given key
    async fn store<T: Serialize + Send + Sync>(&self, key: &str, value: &T, options: Option<StorageOptions>) -> Result<()>;
    
    /// Retrieve and deserialize a value by key
    async fn retrieve<T: DeserializeOwned + Send + Sync>(&self, key: &str, options: Option<StorageOptions>) -> Result<Option<T>>;
    
    /// Delete a value by key
    async fn delete(&self, key: &str, options: Option<StorageOptions>) -> Result<bool>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str, options: Option<StorageOptions>) -> Result<bool>;
    
    /// List keys matching a pattern
    async fn list_keys(&self, pattern: &str, options: Option<StorageOptions>) -> Result<Vec<String>>;
} 