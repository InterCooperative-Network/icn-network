/// Core functionality for the ICN Network
///
/// This crate provides core functionality for the ICN Network,
/// including common traits, data structures, and utilities.

pub mod error;

/// Common Result type for ICN crates
pub type Result<T> = std::result::Result<T, error::Error>;

/// Common module for errors
pub mod error {
    use thiserror::Error;
    
    /// Common error type for ICN crates
    #[derive(Debug, Error)]
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
}

pub mod storage;
pub mod networking;
pub mod crypto;
pub mod config;
pub mod utils;

// Re-export key components
pub use storage::{Storage, StorageResult, StorageError, FileStorage, MemoryStorage};

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
pub async fn init() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{Storage, StorageResult};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::RwLock;
    
    // Simple mock storage implementation for testing
    struct TestStorage {
        data: RwLock<HashMap<String, Vec<u8>>>,
    }
    
    impl TestStorage {
        fn new() -> Self {
            Self {
                data: RwLock::new(HashMap::new()),
            }
        }
    }
    
    #[async_trait::async_trait]
    impl Storage for TestStorage {
        async fn put<T: serde::Serialize + Send + Sync>(
            &self,
            namespace: &str,
            key: &str,
            value: &T,
        ) -> StorageResult<()> {
            let serialized = serde_json::to_vec(value)
                .map_err(|e| StorageError::SerializationError(e.to_string()))?;
            
            let full_key = format!("{}/{}", namespace, key);
            let mut data = self.data.write().await;
            data.insert(full_key, serialized);
            Ok(())
        }
        
        async fn get<T: serde::de::DeserializeOwned + Send + Sync>(
            &self,
            namespace: &str,
            key: &str,
        ) -> StorageResult<T> {
            let full_key = format!("{}/{}", namespace, key);
            let data = self.data.read().await;
            
            let value = data.get(&full_key)
                .ok_or_else(|| StorageError::KeyNotFound(full_key.clone()))?;
                
            serde_json::from_slice(value)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))
        }
        
        async fn contains(&self, namespace: &str, key: &str) -> StorageResult<bool> {
            let full_key = format!("{}/{}", namespace, key);
            let data = self.data.read().await;
            Ok(data.contains_key(&full_key))
        }
        
        async fn delete(&self, namespace: &str, key: &str) -> StorageResult<()> {
            let full_key = format!("{}/{}", namespace, key);
            let mut data = self.data.write().await;
            data.remove(&full_key);
            Ok(())
        }
        
        async fn list_keys(&self, namespace: &str) -> StorageResult<Vec<String>> {
            let data = self.data.read().await;
            let prefix = format!("{}/", namespace);
            
            let keys = data.keys()
                .filter(|k| k.starts_with(&prefix))
                .map(|k| k[prefix.len()..].to_string())
                .collect();
                
            Ok(keys)
        }
    }
    
    #[tokio::test]
    async fn test_storage() {
        let storage = TestStorage::new();
        
        // Test value
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug)]
        struct TestValue {
            name: String,
            value: i32,
        }
        
        let test_value = TestValue {
            name: "test".to_string(),
            value: 42,
        };
        
        // Test put and get
        storage.put("test", "key1", &test_value).await.unwrap();
        let retrieved: TestValue = storage.get("test", "key1").await.unwrap();
        assert_eq!(retrieved, test_value);
        
        // Test contains
        assert!(storage.contains("test", "key1").await.unwrap());
        assert!(!storage.contains("test", "nonexistent").await.unwrap());
        
        // Test list keys
        storage.put("test", "key2", &test_value).await.unwrap();
        let keys = storage.list_keys("test").await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
        
        // Test delete
        storage.delete("test", "key1").await.unwrap();
        assert!(!storage.contains("test", "key1").await.unwrap());
        assert!(storage.contains("test", "key2").await.unwrap());
    }
} 