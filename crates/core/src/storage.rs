//! Storage implementation for ICN core
//!
//! This module provides a storage interface and implementations
//! for ICN components.

use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tokio::fs as async_fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use thiserror::Error;

/// Storage errors
#[derive(Error, Debug, Clone)]
pub enum StorageError {
    /// IO error
    #[error("IO error: {0}")]
    IoError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    /// Key not found
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    
    /// Data corruption
    #[error("Data corruption: {0}")]
    DataCorruption(String),
    
    /// Internal storage error
    #[error("Internal storage error: {0}")]
    InternalError(String),
}

/// Result type for storage operations
pub type StorageResult<T> = Result<T, StorageError>;

/// Storage trait for ICN components
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store a key-value pair
    async fn put<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()>;
    
    /// Get a value by key
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T>;
    
    /// Check if a key exists
    async fn contains(&self, namespace: &str, key: &str) -> StorageResult<bool>;
    
    /// Delete a key-value pair
    async fn delete(&self, namespace: &str, key: &str) -> StorageResult<()>;
    
    /// List all keys in a namespace
    async fn list_keys(&self, namespace: &str) -> StorageResult<Vec<String>>;
}

/// In-memory storage implementation
pub struct MemoryStorage {
    /// Data storage
    data: RwLock<HashMap<String, HashMap<String, Vec<u8>>>>,
}

impl MemoryStorage {
    /// Create a new memory storage
    pub fn new() -> Self {
        Self {
            data: RwLock::new(HashMap::new()),
        }
    }
    
    /// Create a key for the storage
    fn make_key(namespace: &str, key: &str) -> String {
        format!("{}/{}", namespace, key)
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        let serialized = serde_json::to_vec(value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        let mut data = self.data.write().await;
        let ns = data.entry(namespace.to_string()).or_insert_with(HashMap::new);
        ns.insert(key.to_string(), serialized);
        
        Ok(())
    }
    
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        let data = self.data.read().await;
        
        let ns = data.get(namespace)
            .ok_or_else(|| StorageError::KeyNotFound(format!("Namespace {} not found", namespace)))?;
            
        let value = ns.get(key)
            .ok_or_else(|| StorageError::KeyNotFound(format!("Key {} not found in namespace {}", key, namespace)))?;
            
        serde_json::from_slice(value)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))
    }
    
    async fn contains(&self, namespace: &str, key: &str) -> StorageResult<bool> {
        let data = self.data.read().await;
        
        Ok(data.get(namespace)
            .map(|ns| ns.contains_key(key))
            .unwrap_or(false))
    }
    
    async fn delete(&self, namespace: &str, key: &str) -> StorageResult<()> {
        let mut data = self.data.write().await;
        
        if let Some(ns) = data.get_mut(namespace) {
            ns.remove(key);
        }
        
        Ok(())
    }
    
    async fn list_keys(&self, namespace: &str) -> StorageResult<Vec<String>> {
        let data = self.data.read().await;
        
        Ok(data.get(namespace)
            .map(|ns| ns.keys().cloned().collect())
            .unwrap_or_else(Vec::new))
    }
}

/// File-based storage implementation
pub struct FileStorage {
    /// Base directory for storage
    base_dir: PathBuf,
    /// In-memory cache (optional)
    cache: Option<MemoryStorage>,
}

impl FileStorage {
    /// Create a new file storage
    pub fn new(base_dir: &str) -> StorageResult<Self> {
        let path = PathBuf::from(base_dir);
        fs::create_dir_all(&path).map_err(|e| StorageError::IoError(e.to_string()))?;
        
        Ok(Self {
            base_dir: path,
            cache: Some(MemoryStorage::new()),
        })
    }
    
    /// Create a new file storage without caching
    pub fn new_without_cache(base_dir: &str) -> StorageResult<Self> {
        let path = PathBuf::from(base_dir);
        fs::create_dir_all(&path).map_err(|e| StorageError::IoError(e.to_string()))?;
        
        Ok(Self {
            base_dir: path,
            cache: None,
        })
    }
    
    /// Get the path for a namespace
    fn namespace_path(&self, namespace: &str) -> PathBuf {
        self.base_dir.join(namespace)
    }
    
    /// Get the path for a key in a namespace
    fn key_path(&self, namespace: &str, key: &str) -> PathBuf {
        self.namespace_path(namespace).join(format!("{}.json", key))
    }
    
    /// Ensure a namespace directory exists
    async fn ensure_namespace_dir(&self, namespace: &str) -> StorageResult<()> {
        let path = self.namespace_path(namespace);
        
        if !path.exists() {
            async_fs::create_dir_all(&path)
                .await
                .map_err(|e| StorageError::IoError(e.to_string()))?;
        }
        
        Ok(())
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn put<T: Serialize + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        // Ensure namespace directory exists
        self.ensure_namespace_dir(namespace).await?;
        
        // Serialize value
        let serialized = serde_json::to_vec_pretty(value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        
        // Write to file
        let path = self.key_path(namespace, key);
        let mut file = async_fs::File::create(&path)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
            
        file.write_all(&serialized)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
            
        // Update cache if enabled
        if let Some(cache) = &self.cache {
            cache.put(namespace, key, value).await?;
        }
        
        Ok(())
    }
    
    async fn get<T: DeserializeOwned + Send + Sync>(
        &self,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        // Try to get from cache first
        if let Some(cache) = &self.cache {
            if let Ok(value) = cache.get::<T>(namespace, key).await {
                return Ok(value);
            }
        }
        
        // Read from file
        let path = self.key_path(namespace, key);
        
        if !path.exists() {
            return Err(StorageError::KeyNotFound(format!(
                "Key {} not found in namespace {}", key, namespace
            )));
        }
        
        let mut file = async_fs::File::open(&path)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
            
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
            
        // Deserialize
        let value = serde_json::from_slice(&contents)
            .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
            
        // Update cache if enabled
        if let Some(cache) = &self.cache {
            let value_clone: T = serde_json::from_slice(&contents)
                .map_err(|e| StorageError::DeserializationError(e.to_string()))?;
                
            let _ = cache.put(namespace, key, &value_clone).await;
        }
        
        Ok(value)
    }
    
    async fn contains(&self, namespace: &str, key: &str) -> StorageResult<bool> {
        // Check cache first
        if let Some(cache) = &self.cache {
            if let Ok(exists) = cache.contains(namespace, key).await {
                if exists {
                    return Ok(true);
                }
            }
        }
        
        // Check file system
        let path = self.key_path(namespace, key);
        Ok(path.exists())
    }
    
    async fn delete(&self, namespace: &str, key: &str) -> StorageResult<()> {
        // Delete from cache if enabled
        if let Some(cache) = &self.cache {
            let _ = cache.delete(namespace, key).await;
        }
        
        // Delete file
        let path = self.key_path(namespace, key);
        
        if path.exists() {
            async_fs::remove_file(&path)
                .await
                .map_err(|e| StorageError::IoError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    async fn list_keys(&self, namespace: &str) -> StorageResult<Vec<String>> {
        let path = self.namespace_path(namespace);
        
        if !path.exists() {
            return Ok(Vec::new());
        }
        
        // Read directory
        let mut entries = async_fs::read_dir(&path)
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))?;
            
        let mut keys = Vec::new();
        
        while let Some(entry) = entries.next_entry()
            .await
            .map_err(|e| StorageError::IoError(e.to_string()))? {
                
            let path = entry.path();
            
            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(filename) = path.file_stem() {
                    if let Some(key) = filename.to_str() {
                        keys.push(key.to_string());
                    }
                }
            }
        }
        
        Ok(keys)
    }
} 