use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use async_trait::async_trait;
use tracing::{debug, error, trace};

use super::{Storage, StorageError, StorageOptions, StorageResult};

/// A file-based storage implementation
pub struct FileStorage {
    base_path: PathBuf,
    options: StorageOptions,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
}

impl FileStorage {
    /// Create a new file storage instance
    pub async fn new(base_path: impl Into<PathBuf>) -> StorageResult<Self> {
        let path = base_path.into();
        
        // Create base directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path).await?;
        }
        
        Ok(Self {
            base_path: path,
            options: StorageOptions::default(),
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }
    
    /// Set storage options
    pub fn with_options(mut self, options: StorageOptions) -> Self {
        self.options = options;
        self
    }
    
    /// Get the full path for a key
    fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key.replace('/', &std::path::MAIN_SEPARATOR.to_string()))
    }
}

#[async_trait]
impl Storage for FileStorage {
    async fn put(&self, key: &str, data: &[u8]) -> StorageResult<()> {
        let path = self.get_path(key);
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await?;
            }
        }
        
        // Write the data
        let mut file = fs::File::create(&path).await?;
        file.write_all(data).await?;
        
        if self.options.sync_write {
            file.sync_all().await?;
        }
        
        // Update cache if enabled
        if self.options.use_cache {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), data.to_vec());
        }
        
        debug!("Stored data at key: {}", key);
        Ok(())
    }
    
    async fn get(&self, key: &str) -> StorageResult<Vec<u8>> {
        // Check cache first if enabled
        if self.options.use_cache {
            let cache = self.cache.read().await;
            if let Some(data) = cache.get(key) {
                trace!("Retrieved data from cache for key: {}", key);
                return Ok(data.clone());
            }
        }
        
        let path = self.get_path(key);
        if !path.exists() {
            return Err(StorageError::KeyNotFound(key.to_string()));
        }
        
        let mut file = fs::File::open(&path).await?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).await?;
        
        // Update cache if enabled
        if self.options.use_cache {
            let mut cache = self.cache.write().await;
            cache.insert(key.to_string(), data.clone());
        }
        
        debug!("Retrieved data for key: {}", key);
        Ok(data)
    }
    
    async fn delete(&self, key: &str) -> StorageResult<()> {
        let path = self.get_path(key);
        if path.exists() {
            fs::remove_file(path).await?;
            
            // Remove from cache if enabled
            if self.options.use_cache {
                let mut cache = self.cache.write().await;
                cache.remove(key);
            }
            
            debug!("Deleted key: {}", key);
        }
        
        Ok(())
    }
    
    async fn exists(&self, key: &str) -> StorageResult<bool> {
        // Check cache first if enabled
        if self.options.use_cache {
            let cache = self.cache.read().await;
            if cache.contains_key(key) {
                return Ok(true);
            }
        }
        
        let path = self.get_path(key);
        Ok(path.exists())
    }
    
    async fn list(&self, prefix: &str) -> StorageResult<Vec<String>> {
        let path = self.base_path.join(prefix);
        debug!("Listing keys with prefix: {}", prefix);
        
        if !path.exists() {
            return Ok(Vec::new());
        }
        
        if !path.is_dir() {
            return Err(StorageError::NotADirectory(prefix.to_string()));
        }
        
        self.list_directory(&path).await
    }
    
    fn base_path(&self) -> Option<PathBuf> {
        Some(self.base_path.clone())
    }
}

impl FileStorage {
    /// Recursive helper to list directory contents
    async fn list_directory(&self, dir_path: &Path) -> StorageResult<Vec<String>> {
        let mut result = Vec::new();
        
        let mut entries = fs::read_dir(dir_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            if path.is_file() {
                if let Ok(rel_path) = path.strip_prefix(&self.base_path) {
                    let rel_path_str = rel_path.to_string_lossy().to_string();
                    result.push(rel_path_str);
                }
            } else if path.is_dir() {
                // Recursively list subdirectories
                let sub_results = self.list_directory(&path).await?;
                result.extend(sub_results);
            }
        }
        
        Ok(result)
    }
} 