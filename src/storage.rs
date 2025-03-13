use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};

// Storage error types
#[derive(Debug)]
pub enum StorageError {
    IoError(std::io::Error),
    SerializationError(String),
    KeyNotFound(String),
}

impl From<std::io::Error> for StorageError {
    fn from(error: std::io::Error) -> Self {
        StorageError::IoError(error)
    }
}

impl From<serde_json::Error> for StorageError {
    fn from(error: serde_json::Error) -> Self {
        StorageError::SerializationError(error.to_string())
    }
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::IoError(e) => write!(f, "IO error: {}", e),
            StorageError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            StorageError::KeyNotFound(key) => write!(f, "Key not found: {}", key),
        }
    }
}

impl Error for StorageError {}

// Storage options
#[derive(Debug, Clone)]
pub struct StorageOptions {
    pub sync_write: bool,
    pub create_dirs: bool,
}

impl Default for StorageOptions {
    fn default() -> Self {
        StorageOptions {
            sync_write: true,
            create_dirs: true,
        }
    }
}

// Main storage structure
pub struct Storage {
    base_path: PathBuf,
    options: StorageOptions,
    cache: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl Storage {
    // Create a new storage instance
    pub fn new(base_path: &str) -> Result<Self, Box<dyn Error>> {
        let path = PathBuf::from(base_path);
        
        // Create the base directory if it doesn't exist
        if !path.exists() {
            fs::create_dir_all(&path)?;
        }
        
        Ok(Storage {
            base_path: path,
            options: StorageOptions::default(),
            cache: Arc::new(Mutex::new(HashMap::new())),
        })
    }
    
    // Set storage options
    pub fn with_options(mut self, options: StorageOptions) -> Self {
        self.options = options;
        self
    }
    
    // Get the full path for a key
    fn get_path(&self, key: &str) -> PathBuf {
        let mut path = self.base_path.clone();
        
        // Split the key by / and create subdirectories
        let parts: Vec<&str> = key.split('/').collect();
        
        // Add all parts except the last one as subdirectories
        for part in &parts[..parts.len() - 1] {
            path.push(part);
        }
        
        // Create directories if needed
        if self.options.create_dirs && parts.len() > 1 {
            let _ = fs::create_dir_all(&path);
        }
        
        // Add the last part as the filename
        path.push(parts[parts.len() - 1]);
        
        path
    }
    
    // Put a value
    pub fn put(&self, key: &str, value: &[u8]) -> Result<(), Box<dyn Error>> {
        let path = self.get_path(key);
        
        // Make sure the parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        // Write the file
        let mut file = File::create(&path)?;
        file.write_all(value)?;
        
        // Sync if needed
        if self.options.sync_write {
            file.sync_all()?;
        }
        
        // Update cache
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.to_string(), value.to_vec());
        
        Ok(())
    }
    
    // Put a serializable value
    pub fn put_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>> {
        let json = serde_json::to_vec(value)?;
        self.put(key, &json)?;
        Ok(())
    }
    
    // Get a value
    pub fn get(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        // Check cache first
        {
            let cache = self.cache.lock().unwrap();
            if let Some(value) = cache.get(key) {
                return Ok(value.clone());
            }
        }
        
        // Read from file
        let path = self.get_path(key);
        if !path.exists() {
            return Err(Box::new(StorageError::KeyNotFound(key.to_string())));
        }
        
        let mut file = File::open(&path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        // Update cache
        let mut cache = self.cache.lock().unwrap();
        cache.insert(key.to_string(), buffer.clone());
        
        Ok(buffer)
    }
    
    // Get a deserializable value
    pub fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, Box<dyn Error>> {
        let data = self.get(key)?;
        let value = serde_json::from_slice(&data)?;
        Ok(value)
    }
    
    // Delete a value
    pub fn delete(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let path = self.get_path(key);
        
        // Delete the file if it exists
        if path.exists() {
            fs::remove_file(&path)?;
        }
        
        // Remove from cache
        let mut cache = self.cache.lock().unwrap();
        cache.remove(key);
        
        Ok(())
    }
    
    // List keys with a prefix
    pub fn list(&self, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let mut prefix_path = self.base_path.clone();
        
        // Split the prefix by / and navigate to that directory
        let parts: Vec<&str> = prefix.split('/').collect();
        for part in &parts {
            if !part.is_empty() {
                prefix_path.push(part);
            }
        }
        
        // If the prefix path doesn't exist, return an empty list
        if !prefix_path.exists() {
            return Ok(Vec::new());
        }
        
        // Collect all files recursively
        let mut keys = Vec::new();
        self.collect_keys(&prefix_path, &mut keys, prefix)?;
        
        Ok(keys)
    }
    
    // Helper to recursively collect keys
    fn collect_keys(&self, dir: &Path, keys: &mut Vec<String>, prefix: &str) -> Result<(), Box<dyn Error>> {
        if !dir.exists() || !dir.is_dir() {
            return Ok(());
        }
        
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                // Recurse into subdirectories
                self.collect_keys(&path, keys, prefix)?;
            } else {
                // Convert the path to a key
                if let Ok(key) = path.strip_prefix(&self.base_path) {
                    if let Some(key_str) = key.to_str() {
                        // Only include keys that start with the prefix
                        if key_str.starts_with(prefix) {
                            keys.push(key_str.to_string());
                        }
                    }
                }
            }
        }
        
        Ok(())
    }
} 