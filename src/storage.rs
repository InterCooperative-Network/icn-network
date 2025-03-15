use std::collections::HashMap;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

// Storage error types
#[derive(Debug)]
pub enum StorageError {
    IoError(std::io::Error),
    SerializationError(String),
    KeyNotFound(String),
    NotADirectory(String),
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
            StorageError::NotADirectory(dir) => write!(f, "Not a directory: {}", dir),
        }
    }
}

impl Error for StorageError {}

// Storage options
#[derive(Debug, Clone)]
pub struct StorageOptions {
    pub sync_write: bool,
    pub create_dirs: bool,
    pub use_cache: bool,
}

impl Default for StorageOptions {
    fn default() -> Self {
        StorageOptions {
            sync_write: true,
            create_dirs: true,
            use_cache: true,
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
    pub fn new(base_path: &str) -> Self {
        let path = PathBuf::from(base_path);
        if !path.exists() {
            fs::create_dir_all(&path).unwrap();
        }
        Storage {
            base_path: path,
            options: StorageOptions::default(),
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    // Set storage options
    pub fn with_options(mut self, options: StorageOptions) -> Self {
        self.options = options;
        self
    }
    
    // Get the full path for a key
    pub fn get_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key.replace("/", &std::path::MAIN_SEPARATOR.to_string()))
    }
    
    // Get the base path as a string
    pub fn get_base_path(&self) -> Result<String, Box<dyn Error>> {
        match self.base_path.to_str() {
            Some(path) => Ok(path.to_string()),
            None => Err(Box::new(StorageError::IoError(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Base path contains invalid UTF-8 characters"
            ))))
        }
    }
    
    // Put data into storage
    pub fn put(&self, key: &str, data: &[u8]) -> Result<(), Box<dyn Error>> {
        let path = self.get_path(key);
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        // Write the data
        fs::write(&path, data)?;
        
        // Update cache if enabled
        if self.options.use_cache {
            let mut cache = self.cache.lock().unwrap();
            cache.insert(key.to_string(), data.to_vec());
        }
        
        Ok(())
    }
    
    // Get data from storage
    pub fn get(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let path = self.get_path(key);
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(data)
    }
    
    // Put JSON data into storage
    pub fn put_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>> {
        let json_data = serde_json::to_vec_pretty(value)?;
        self.put(key, &json_data)
    }
    
    // Load JSON data
    pub fn get_json<T: DeserializeOwned>(&self, key: &str) -> Result<T, Box<dyn Error>> {
        let data = self.get(key)?;
        let value = serde_json::from_slice(&data)?;
        Ok(value)
    }
    
    // Alternative method names to maintain compatibility
    pub fn store_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), Box<dyn Error>> {
        self.put_json(key, value)
    }
    
    pub fn load_json<T: DeserializeOwned>(&self, key: &str) -> Result<T, Box<dyn Error>> {
        self.get_json(key)
    }
    
    // Delete data
    pub fn delete(&self, key: &str) -> Result<(), Box<dyn Error>> {
        let path = self.get_path(key);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }
    
    // Check if key exists
    pub fn exists(&self, key: &str) -> bool {
        self.get_path(key).exists()
    }
    
    // List all keys with a given prefix
    pub fn list(&self, key: &str) -> Result<Vec<String>, Box<dyn Error>> {
        let path = Path::new(&self.base_path).join(key);
        println!("Listing files at path: {:?}", path);
        
        if path.is_dir() {
            println!("Path is a directory, calling list_directory");
            self.list_directory(&path)
        } else {
            println!("Path is not a directory: {:?}", path);
            Err(Box::new(StorageError::NotADirectory(key.to_string())))
        }
    }
    
    // List files for compatibility
    pub fn list_files(&self, prefix: &str) -> Result<Vec<String>, Box<dyn Error>> {
        self.list(prefix)
    }
    
    // List keys in a directory
    fn list_directory(&self, dir_path: &Path) -> Result<Vec<String>, Box<dyn Error>> {
        println!("Listing directory: {:?}", dir_path);
        let mut result = Vec::new();
        
        match fs::read_dir(dir_path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => {
                            let path = entry.path();
                            println!("Found entry: {:?}", path);
                            
                            if path.is_file() {
                                if let Some(rel_path) = path.strip_prefix(&self.base_path).ok() {
                                    let rel_path_str = rel_path.to_string_lossy().to_string();
                                    println!("Adding file to result: {}", rel_path_str);
                                    result.push(rel_path_str);
                                }
                            } else if path.is_dir() {
                                println!("Found subdirectory: {:?}", path);
                                // Recursively list subdirectories
                                if let Some(rel_path) = path.strip_prefix(&self.base_path).ok() {
                                    let sub_results = self.list_directory(&path)?;
                                    for sub_path in sub_results {
                                        result.push(sub_path);
                                    }
                                }
                            }
                        },
                        Err(e) => {
                            println!("Error reading directory entry: {:?}", e);
                            // Continue with other entries
                        }
                    }
                }
                
                println!("Found {} files in directory {:?}", result.len(), dir_path);
                Ok(result)
            },
            Err(e) => {
                println!("Error reading directory {:?}: {:?}", dir_path, e);
                Err(Box::new(e))
            }
        }
    }

    // Add a clone method for our tests
    pub fn clone(&self) -> Self {
        Storage {
            base_path: self.base_path.clone(),
            options: StorageOptions {
                sync_write: self.options.sync_write,
                create_dirs: self.options.create_dirs,
                use_cache: self.options.use_cache,
            },
            cache: self.cache.clone(),
        }
    }
} 