/// Core functionality for the ICN Network
///
/// This crate provides core functionality for the ICN Network,
/// including common traits, data structures, and utilities.

/// Common Error type for ICN crates
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Error from I/O operation
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    
    /// Error from serialization or deserialization
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    
    /// Error from parsing
    #[error("Parse error: {0}")]
    Parse(String),
    
    /// Error from network operation
    #[error("Network error: {0}")]
    Network(String),
    
    /// Error from storage operation
    #[error("Storage error: {0}")]
    Storage(String),
    
    /// Error from economic operation
    #[error("Economic error: {0}")]
    Economic(String),
    
    /// Error from governance operation
    #[error("Governance error: {0}")]
    Governance(String),
    
    /// Error from DSL operation
    #[error("DSL error: {0}")]
    Dsl(String),
}

/// Common Result type for ICN crates
pub type Result<T> = std::result::Result<T, Error>;

pub mod crypto;
pub mod storage;
pub mod utils;
pub mod networking;
pub mod identity;
pub mod common;
pub mod config;

// Re-export common types
pub use common::{
    CommonError,
    Result,
    DID,
    Value,
    EntityId,
    Timestamp,
    OperationContext,
};

// Re-export key components
pub use storage::Storage;
pub use crypto::{CryptoUtils, Hash, Signature};
pub use identity::{Identity, DidDocument, Credential};
pub use config::{NodeConfig, TlsConfig, ConfigError};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
/// Package description
pub const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/// Initialize tracing for ICN
pub fn init_tracing() {
    use tracing_subscriber::FmtSubscriber;
    
    // Initialize the default tracing subscriber
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .finish();
    
    // Set the subscriber as the global default
    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set global tracing subscriber");
}

// Core functionality for the ICN system

/// Common utilities
pub mod common {
    /// Common types for ICN
    pub mod types {
        /// A simple type alias for a hash
        pub type Hash = String;
    }
}

/// Initialize ICN core system
pub async fn init() -> Result<()> {
    init_tracing();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Storage, MemoryStorage, VersioningManager, VersionInfo};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Test implementations and code
    
    #[tokio::test]
    async fn test_versioning_integration() {
        // Setup storage and versioning
        let storage = Arc::new(MemoryStorage::new());
        let versioning_manager = VersioningManager::new(storage.clone(), 5);
        
        // Generate test key and version ID
        let test_key = "test-versioned-doc";
        let version_id = versioning_manager.generate_version_id();
        
        // Create version storage key and store some data
        let version_storage_key = versioning_manager.create_version_storage_key(test_key, &version_id);
        let test_data = b"This is version 1 of the document";
        storage.put(&version_storage_key, test_data).await.unwrap();
        
        // Create metadata for the version
        let metadata = HashMap::new();
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Create version info
        let version_info = VersionInfo {
            version_id: version_id.clone(),
            created_at: timestamp,
            size_bytes: test_data.len() as u64,
            metadata,
            storage_key: version_storage_key,
            content_hash: "fakehash123".to_string(),
            created_by: "test-user".to_string(),
            comment: Some("Initial version".to_string()),
        };
        
        // Initialize versioning with the version info
        versioning_manager.init_versioning(test_key, &version_id, version_info)
            .await
            .unwrap();
        
        // Get version history and verify
        let history = versioning_manager.get_version_history(test_key).await.unwrap();
        assert_eq!(history.version_count(), 1);
        assert_eq!(history.current_version_id, Some(version_id));
        
        // Clean up
        versioning_manager.delete_all_versions(test_key).await.unwrap();
    }
} 