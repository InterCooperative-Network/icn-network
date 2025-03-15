//! Serialization utilities
//!
//! This module provides common serialization and deserialization functions.

use std::path::{Path, PathBuf};
use std::io::{self, Read, Write};
use tokio::fs;
use serde::{Serialize, Deserialize};
use super::UtilError;

/// JSON serialization helper for Storage trait implementations
pub trait JsonStorage {
    /// Save a serializable value to the store
    async fn put_json<T: Serialize>(&self, key: &str, value: &T) -> Result<(), UtilError>;
    
    /// Get a deserialized value from the store
    async fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T, UtilError>;
}

/// Serialize an object to JSON
pub fn to_json<T: Serialize>(value: &T) -> Result<String, UtilError> {
    serde_json::to_string(value)
        .map_err(|e| UtilError::ParseError(format!("Failed to serialize to JSON: {}", e)))
}

/// Serialize an object to pretty JSON
pub fn to_json_pretty<T: Serialize>(value: &T) -> Result<String, UtilError> {
    serde_json::to_string_pretty(value)
        .map_err(|e| UtilError::ParseError(format!("Failed to serialize to pretty JSON: {}", e)))
}

/// Deserialize an object from JSON
pub fn from_json<T: for<'de> Deserialize<'de>>(json: &str) -> Result<T, UtilError> {
    serde_json::from_str(json)
        .map_err(|e| UtilError::ParseError(format!("Failed to deserialize from JSON: {}", e)))
}

/// Serialize to TOML
pub fn to_toml<T: Serialize>(value: &T) -> Result<String, UtilError> {
    toml::to_string(value)
        .map_err(|e| UtilError::ParseError(format!("Failed to serialize to TOML: {}", e)))
}

/// Serialize to pretty TOML
pub fn to_toml_pretty<T: Serialize>(value: &T) -> Result<String, UtilError> {
    toml::to_string_pretty(value)
        .map_err(|e| UtilError::ParseError(format!("Failed to serialize to pretty TOML: {}", e)))
}

/// Deserialize from TOML
pub fn from_toml<T: for<'de> Deserialize<'de>>(toml_str: &str) -> Result<T, UtilError> {
    toml::from_str(toml_str)
        .map_err(|e| UtilError::ParseError(format!("Failed to deserialize from TOML: {}", e)))
}

/// Serialize to bytes using bincode
pub fn to_bincode<T: Serialize>(value: &T) -> Result<Vec<u8>, UtilError> {
    bincode::serialize(value)
        .map_err(|e| UtilError::ParseError(format!("Failed to serialize to bincode: {}", e)))
}

/// Deserialize from bytes using bincode
pub fn from_bincode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> Result<T, UtilError> {
    bincode::deserialize(bytes)
        .map_err(|e| UtilError::ParseError(format!("Failed to deserialize from bincode: {}", e)))
}

/// Save data to a file
pub async fn save_to_file<P: AsRef<Path>>(path: P, data: &[u8]) -> io::Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.as_ref().parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).await?;
        }
    }
    
    fs::write(path, data).await
}

/// Load data from a file
pub async fn load_from_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
    fs::read(path).await
}

/// Save a serializable object to a JSON file
pub async fn save_json_to_file<T: Serialize, P: AsRef<Path>>(path: P, value: &T) -> Result<(), UtilError> {
    let json = to_json_pretty(value)?;
    save_to_file(path, json.as_bytes())
        .await
        .map_err(|e| UtilError::IoError(e))
}

/// Load a serializable object from a JSON file
pub async fn load_json_from_file<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(path: P) -> Result<T, UtilError> {
    let data = load_from_file(path)
        .await
        .map_err(|e| UtilError::IoError(e))?;
    
    let json = String::from_utf8(data)
        .map_err(|e| UtilError::ParseError(format!("Invalid UTF-8: {}", e)))?;
    
    from_json(&json)
}

/// Save a serializable object to a TOML file
pub async fn save_toml_to_file<T: Serialize, P: AsRef<Path>>(path: P, value: &T) -> Result<(), UtilError> {
    let toml_str = to_toml_pretty(value)?;
    save_to_file(path, toml_str.as_bytes())
        .await
        .map_err(|e| UtilError::IoError(e))
}

/// Load a serializable object from a TOML file
pub async fn load_toml_from_file<T: for<'de> Deserialize<'de>, P: AsRef<Path>>(path: P) -> Result<T, UtilError> {
    let data = load_from_file(path)
        .await
        .map_err(|e| UtilError::IoError(e))?;
    
    let toml_str = String::from_utf8(data)
        .map_err(|e| UtilError::ParseError(format!("Invalid UTF-8: {}", e)))?;
    
    from_toml(&toml_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Serialize, Deserialize};
    
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        value: i32,
        tags: Vec<String>,
    }
    
    #[test]
    fn test_json_serialization() {
        let test = TestStruct {
            name: "test".to_string(),
            value: 42,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        
        let json = to_json(&test).unwrap();
        let parsed: TestStruct = from_json(&json).unwrap();
        
        assert_eq!(test, parsed);
    }
    
    #[test]
    fn test_toml_serialization() {
        let test = TestStruct {
            name: "test".to_string(),
            value: 42,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        
        let toml_str = to_toml(&test).unwrap();
        let parsed: TestStruct = from_toml(&toml_str).unwrap();
        
        assert_eq!(test, parsed);
    }
    
    #[test]
    fn test_bincode_serialization() {
        let test = TestStruct {
            name: "test".to_string(),
            value: 42,
            tags: vec!["tag1".to_string(), "tag2".to_string()],
        };
        
        let bytes = to_bincode(&test).unwrap();
        let parsed: TestStruct = from_bincode(&bytes).unwrap();
        
        assert_eq!(test, parsed);
    }
} 