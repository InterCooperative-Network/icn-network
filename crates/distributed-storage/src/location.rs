use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::{policy::DataAccessPolicy, encryption::EncryptionMetadata};

/// Information about where data is stored in the network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLocation {
    /// Key of the stored data
    pub key: String,
    /// List of storage peer IDs that have a copy
    pub storage_peers: Vec<String>,
    /// Access policy for the data
    pub policy: DataAccessPolicy,
    /// Hash of the data content
    pub content_hash: String,
    /// Size of the data in bytes
    pub size_bytes: u64,
    /// Unix timestamp when the data was created
    pub created_at: u64,
    /// Unix timestamp when the data was last updated
    pub updated_at: u64,
    /// Encryption metadata if the data is encrypted
    pub encryption_metadata: Option<EncryptionMetadata>,
    /// Whether this is a versioned object
    pub is_versioned: bool,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl DataLocation {
    /// Create a new data location
    pub fn new(
        key: String,
        storage_peers: Vec<String>,
        policy: DataAccessPolicy,
        content_hash: String,
        size_bytes: u64,
        created_at: u64,
    ) -> Self {
        Self {
            key,
            storage_peers,
            policy,
            content_hash,
            size_bytes,
            created_at,
            updated_at: created_at,
            encryption_metadata: None,
            is_versioned: false,
            metadata: HashMap::new(),
        }
    }
    
    /// Add a storage peer
    pub fn add_peer(&mut self, peer_id: String) {
        if !self.storage_peers.contains(&peer_id) {
            self.storage_peers.push(peer_id);
        }
    }
    
    /// Remove a storage peer
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.storage_peers.retain(|id| id != peer_id);
    }
    
    /// Update the content hash and size
    pub fn update_content(&mut self, content_hash: String, size_bytes: u64, updated_at: u64) {
        self.content_hash = content_hash;
        self.size_bytes = size_bytes;
        self.updated_at = updated_at;
    }
    
    /// Set encryption metadata
    pub fn set_encryption_metadata(&mut self, metadata: EncryptionMetadata) {
        self.encryption_metadata = Some(metadata);
    }
    
    /// Clear encryption metadata
    pub fn clear_encryption_metadata(&mut self) {
        self.encryption_metadata = None;
    }
    
    /// Set versioning status
    pub fn set_versioned(&mut self, is_versioned: bool) {
        self.is_versioned = is_versioned;
    }
    
    /// Add or update metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Remove metadata
    pub fn remove_metadata(&mut self, key: &str) -> Option<String> {
        self.metadata.remove(key)
    }
    
    /// Get the number of replicas
    pub fn replica_count(&self) -> usize {
        self.storage_peers.len()
    }
    
    /// Check if the data meets the redundancy requirements
    pub fn has_sufficient_replicas(&self) -> bool {
        self.replica_count() >= self.policy.redundancy_factor as usize
    }
    
    /// Check if the data has expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        self.policy.is_expired(current_time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_location() -> DataLocation {
        DataLocation::new(
            "test-key".to_string(),
            vec!["peer1".to_string(), "peer2".to_string()],
            DataAccessPolicy::default(),
            "hash123".to_string(),
            1000,
            1000,
        )
    }
    
    #[test]
    fn test_peer_management() {
        let mut location = create_test_location();
        
        // Test initial state
        assert_eq!(location.replica_count(), 2);
        
        // Test adding a peer
        location.add_peer("peer3".to_string());
        assert_eq!(location.replica_count(), 3);
        
        // Test adding a duplicate peer
        location.add_peer("peer3".to_string());
        assert_eq!(location.replica_count(), 3);
        
        // Test removing a peer
        location.remove_peer("peer2");
        assert_eq!(location.replica_count(), 2);
        assert!(!location.storage_peers.contains(&"peer2".to_string()));
    }
    
    #[test]
    fn test_content_updates() {
        let mut location = create_test_location();
        
        // Test initial state
        assert_eq!(location.content_hash, "hash123");
        assert_eq!(location.size_bytes, 1000);
        assert_eq!(location.updated_at, 1000);
        
        // Test updating content
        location.update_content("newhash".to_string(), 2000, 2000);
        assert_eq!(location.content_hash, "newhash");
        assert_eq!(location.size_bytes, 2000);
        assert_eq!(location.updated_at, 2000);
    }
    
    #[test]
    fn test_encryption_metadata() {
        let mut location = create_test_location();
        
        // Test initial state
        assert!(location.encryption_metadata.is_none());
        
        // Test setting metadata
        let metadata = EncryptionMetadata {
            key_id: "key1".to_string(),
            iv: vec![1, 2, 3],
            tag: vec![4, 5, 6],
            encryption_type: "aes-256-gcm".to_string(),
        };
        location.set_encryption_metadata(metadata.clone());
        assert!(location.encryption_metadata.is_some());
        assert_eq!(location.encryption_metadata.as_ref().unwrap().key_id, "key1");
        
        // Test clearing metadata
        location.clear_encryption_metadata();
        assert!(location.encryption_metadata.is_none());
    }
    
    #[test]
    fn test_metadata() {
        let mut location = create_test_location();
        
        // Test initial state
        assert!(location.metadata.is_empty());
        
        // Test setting metadata
        location.set_metadata("key1".to_string(), "value1".to_string());
        assert_eq!(location.metadata.get("key1").unwrap(), "value1");
        
        // Test updating metadata
        location.set_metadata("key1".to_string(), "value2".to_string());
        assert_eq!(location.metadata.get("key1").unwrap(), "value2");
        
        // Test removing metadata
        let removed = location.remove_metadata("key1");
        assert_eq!(removed.unwrap(), "value2");
        assert!(!location.metadata.contains_key("key1"));
    }
    
    #[test]
    fn test_replica_requirements() {
        let mut location = create_test_location();
        
        // Test with default redundancy factor (3)
        assert!(!location.has_sufficient_replicas());
        
        // Add more peers to meet requirements
        location.add_peer("peer3".to_string());
        assert!(location.has_sufficient_replicas());
        
        // Test with custom redundancy factor
        location.policy.set_redundancy_factor(4);
        assert!(!location.has_sufficient_replicas());
        
        location.add_peer("peer4".to_string());
        assert!(location.has_sufficient_replicas());
    }
    
    #[test]
    fn test_expiration() {
        let mut location = create_test_location();
        
        // Test with no expiration
        assert!(!location.is_expired(2000));
        
        // Test with expiration
        location.policy.set_expiration_time(Some(1500));
        assert!(!location.is_expired(1000));
        assert!(location.is_expired(1500));
        assert!(location.is_expired(2000));
    }
} 