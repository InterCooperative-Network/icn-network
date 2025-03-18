use super::primitives::*;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::error::Error;
use std::path::PathBuf;
use tokio::fs;
use tokio::io;

/// Trait defining storage operations for governance state
#[async_trait]
pub trait GovernanceStore: Send + Sync {
    /// Store a serializable value with the given key
    async fn put<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>>;
    
    /// Retrieve and deserialize a value by key
    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>, Box<dyn Error>>;
    
    /// List all keys with a given prefix
    async fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn Error>>;
    
    /// Delete a key and its associated value
    async fn delete(&self, key: &str) -> Result<(), Box<dyn Error>>;
    
    /// Atomic compare-and-swap operation
    async fn cas<T: Serialize + DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
        expected: &T,
        new: &T
    ) -> Result<bool, Box<dyn Error>>;
}

/// Simple file-based implementation of GovernanceStore
pub struct FileStore {
    root_dir: PathBuf,
}

impl FileStore {
    pub fn new(root_dir: PathBuf) -> io::Result<Self> {
        fs::create_dir_all(&root_dir).blocking_ok()?;
        Ok(Self { root_dir })
    }

    fn key_to_path(&self, key: &str) -> PathBuf {
        let mut path = self.root_dir.clone();
        for segment in key.split('/') {
            path.push(segment);
        }
        path.set_extension("json");
        path
    }
}

#[async_trait]
impl GovernanceStore for FileStore {
    async fn put<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>> {
        let path = self.key_to_path(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        let json = serde_json::to_string_pretty(value)?;
        fs::write(path, json).await?;
        Ok(())
    }

    async fn get<T: DeserializeOwned + Send + Sync>(&self, key: &str) -> Result<Option<T>, Box<dyn Error>> {
        let path = self.key_to_path(key);
        match fs::read_to_string(path).await {
            Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut prefix_path = self.root_dir.clone();
        for segment in prefix.split('/') {
            prefix_path.push(segment);
        }

        let mut keys = Vec::new();
        let mut entries = fs::read_dir(prefix_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                if name.ends_with(".json") {
                    keys.push(name[..name.len()-5].to_string());
                }
            }
        }
        Ok(keys)
    }

    async fn delete(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let path = self.key_to_path(key);
        match fs::remove_file(path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn cas<T: Serialize + DeserializeOwned + Send + Sync>(
        &self,
        key: &str,
        expected: &T,
        new: &T
    ) -> Result<bool, Box<dyn Error>> {
        let path = self.key_to_path(key);
        
        // Read current value
        let current = match fs::read_to_string(&path).await {
            Ok(json) => Some(serde_json::from_str(&json)?),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            Err(e) => return Err(Box::new(e)),
        };

        // Compare with expected
        if serde_json::to_string(expected)? == serde_json::to_string(&current)? {
            // Write new value
            let json = serde_json::to_string_pretty(new)?;
            fs::write(path, json).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
} 