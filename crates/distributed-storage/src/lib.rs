//! Distributed storage implementation for the Intercooperative Network
//!
//! This crate provides distributed storage functionality, including:
//! - Peer discovery and management
//! - Data replication and redundancy
//! - Access control and federation policies
//! - Encryption and key management
//! - Versioning support
//! - Quota management

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use storage::{Storage, StorageError, StorageResult};

pub mod peer;
pub mod policy;
pub mod encryption;
pub mod location;
pub mod dht;

// Re-export commonly used types
pub use peer::StoragePeer;
pub use policy::DataAccessPolicy;
pub use encryption::{StorageEncryptionService, EncryptionMetadata, EncryptionError};
pub use location::DataLocation;
pub use dht::{DistributedHashTable, DhtError, DhtResult};

/// Access type for data operations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessType {
    Read,
    Write,
    Admin,
}

/// Result type for distributed storage operations
pub type DistributedStorageResult<T> = Result<T, DistributedStorageError>;

/// Errors that can occur in distributed storage operations
#[derive(Debug, Error)]
pub enum DistributedStorageError {
    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),
    
    #[error("DHT error: {0}")]
    Dht(#[from] DhtError),
    
    #[error("Encryption error: {0}")]
    Encryption(#[from] EncryptionError),
    
    #[error("Access denied: {0}")]
    AccessDenied(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Federation error: {0}")]
    Federation(String),
    
    #[error("Quota exceeded: {0}")]
    QuotaExceeded(String),
    
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Core distributed storage interface
#[async_trait::async_trait]
pub trait DistributedStorage: Send + Sync + 'static {
    /// Store data with the given key and policy
    async fn put(&self, key: &str, data: &[u8], policy: DataAccessPolicy) -> DistributedStorageResult<()>;
    
    /// Retrieve data by key
    async fn get(&self, key: &str) -> DistributedStorageResult<Vec<u8>>;
    
    /// Delete data by key
    async fn delete(&self, key: &str) -> DistributedStorageResult<()>;
    
    /// Check if a key exists
    async fn exists(&self, key: &str) -> DistributedStorageResult<bool>;
    
    /// List keys with a given prefix
    async fn list(&self, prefix: &str) -> DistributedStorageResult<Vec<String>>;
    
    /// Get the access policy for a key
    async fn get_policy(&self, key: &str) -> DistributedStorageResult<DataAccessPolicy>;
    
    /// Update the access policy for a key
    async fn update_policy(&self, key: &str, policy: DataAccessPolicy) -> DistributedStorageResult<()>;
    
    /// Get information about where data is stored
    async fn get_location(&self, key: &str) -> DistributedStorageResult<DataLocation>;
    
    /// Check if the current node has access to perform an operation
    async fn check_access(&self, key: &str, access_type: AccessType) -> DistributedStorageResult<bool>;
}

/// Helper function to compute a hash of data
pub(crate) fn compute_hash(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
