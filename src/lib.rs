#![recursion_limit = "256"]

//! Intercooperative Network (ICN) - A decentralized infrastructure for cooperative economies
//!
//! This crate provides the core functionality for the Intercooperative Network,
//! a decentralized infrastructure designed to support cooperative economic activities.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use log::{info, error};
use std::sync::Arc;
use serde::{Serialize, de::DeserializeOwned};
use async_trait::async_trait;

// Add trait extension for icn_core::storage::Storage
// This will provide the necessary functionality for get_json and put_json methods
#[async_trait]
pub trait JsonStorage {
    /// Store a serializable value at the specified key
    async fn put_json<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>>;
    
    /// Retrieve and deserialize a value from the specified key
    async fn get_json<T: DeserializeOwned + Send>(&self, key: &str) -> Result<T, Box<dyn Error>>;
}

#[async_trait]
impl JsonStorage for Arc<dyn icn_core::storage::Storage> {
    async fn put_json<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>> {
        let json_data = serde_json::to_vec_pretty(value)?;
        self.put(key, &json_data).await.map_err(|e| e.into())
    }
    
    async fn get_json<T: DeserializeOwned + Send>(&self, key: &str) -> Result<T, Box<dyn Error>> {
        let data = self.get(key).await.map_err(|e| Box::new(e) as Box<dyn Error>)?;
        serde_json::from_slice(&data).map_err(|e| e.into())
    }
}

// Public modules
// pub mod identity;
pub mod storage;
pub mod crypto;
pub mod resource_sharing;
pub mod cross_federation_governance;
pub mod federation_governance;
pub mod federation;
pub mod reputation;
pub mod distributed_storage;
pub mod federation_storage_router;
pub mod economic;

// Temporarily disabled modules due to missing files
// We'll address these in a future update
// pub mod networking;
// pub mod error;
// pub mod economics;
// pub mod governance;
// pub mod integration;

/// Module version information
pub mod version {
    /// Version of the ICN implementation
    pub const VERSION: &str = env!("CARGO_PKG_VERSION");
    
    /// Major version number
    pub const MAJOR: u32 = 0;
    
    /// Minor version number
    pub const MINOR: u32 = 1;
    
    /// Patch version number
    pub const PATCH: u32 = 0;
}

/// Run the ICN node with configuration from environment variables or config file
pub async fn run_node() -> Result<(), Box<dyn Error>> {
    info!("Starting ICN Node v{}", version::VERSION);
    
    // Load configuration from environment or file
    let config_path = env::var("ICN_CONFIG_FILE").unwrap_or_else(|_| "/etc/icn/node.yaml".to_string());
    
    info!("Using config from: {}", config_path);
    
    // Check if config exists
    if !Path::new(&config_path).exists() {
        error!("Configuration file not found: {}", config_path);
        return Err("Configuration file not found".into());
    }
    
    // Load and parse configuration
    let _config_content = fs::read_to_string(&config_path)?;
    
    // Display node information
    info!("Node ID: {}", env::var("ICN_NODE_ID").unwrap_or_else(|_| "unknown".to_string()));
    info!("Cooperative ID: {}", env::var("ICN_COOP_ID").unwrap_or_else(|_| "unknown".to_string()));
    
    // In a real implementation, we would initialize and run the node here
    // For now, we'll just keep the process alive
    info!("Node initialized and running");
    
    // Sleep to keep the node running
    tokio::signal::ctrl_c().await?;
    info!("Shutting down node");
    
    Ok(())
}

// Export the core types we need
// pub use identity::Identity;
pub use icn_core::storage::Storage;
pub use federation_governance::*;
pub use cross_federation_governance::*;
pub use reputation::ReputationSystem;
pub use distributed_storage::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType};
pub use federation_storage_router::{FederationStorageRouter, StorageRoute};
pub use icn_economic::{EconomicSystem, FederationEconomicConfig, EconomicError};
pub use icn_mutual_credit::{Transaction, TransactionType, TransactionStatus, Amount, CreditLimit, Account};

#[cfg(test)]
mod tests {
    use crate::storage::versioning::{VersioningManager, VersionInfo};
    use crate::storage::memory_storage::MemoryStorage;
    use std::collections::HashMap;
    use std::sync::Arc;
    
    #[tokio::test]
    async fn test_versioning_integration() {
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
