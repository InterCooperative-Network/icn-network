//! Distributed Hash Table implementation for the overlay network
//!
//! This module provides a DHT for storing and retrieving data in the overlay network.

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use tracing::{debug, info};

use crate::error::{Result, NetworkError};
use super::address::OverlayAddress;

/// A key for the DHT
pub type Key = Vec<u8>;

/// A value stored in the DHT
pub type Value = Vec<u8>;

/// Information about a node in the overlay network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node identifier
    pub id: String,
    /// Node's overlay address
    pub address: OverlayAddress,
    /// Last time this node was seen (timestamp)
    pub last_seen: i64,
}

/// A Kademlia-inspired distributed hash table for the overlay network
pub struct DistributedHashTable {
    /// Local node ID
    node_id: String,
    /// Local overlay address
    local_address: Option<OverlayAddress>,
    /// K-buckets for node routing
    buckets: Vec<Vec<NodeInfo>>,
    /// Stored key-value pairs
    storage: HashMap<Key, Value>,
}

impl DistributedHashTable {
    /// Create a new DHT
    pub fn new() -> Self {
        Self {
            node_id: String::new(),
            local_address: None,
            buckets: vec![Vec::new(); 256], // 256 k-buckets for IPv6-like addresses
            storage: HashMap::new(),
        }
    }
    
    /// Initialize the DHT
    pub fn initialize(&mut self, node_id: &str, local_address: &OverlayAddress) -> Result<()> {
        self.node_id = node_id.to_string();
        self.local_address = Some(local_address.clone());
        
        info!("Initialized DHT for node {} with address {:?}", node_id, local_address);
        Ok(())
    }
    
    /// Store a key-value pair in the DHT
    pub fn store(&mut self, key: Key, value: Value) -> Result<()> {
        // In a real implementation, this would find the appropriate nodes to store the value
        // For now, just store locally
        self.storage.insert(key.clone(), value.clone());
        debug!("Stored value for key {:?}", key);
        
        Ok(())
    }
    
    /// Retrieve a value from the DHT
    pub fn get(&self, key: &Key) -> Result<Value> {
        // In a real implementation, this would query other nodes if not found locally
        // For now, just check local storage
        if let Some(value) = self.storage.get(key) {
            debug!("Found value for key {:?}", key);
            return Ok(value.clone());
        }
        
        Err(NetworkError::Other(format!("Key not found in DHT: {:?}", key)))
    }
    
    /// Add a node to the appropriate k-bucket
    pub fn add_node(&mut self, node: NodeInfo) -> Result<()> {
        if let Some(local_addr) = &self.local_address {
            // Calculate distance between local address and node address
            let distance = self.calculate_distance(local_addr, &node.address);
            
            // Find the appropriate bucket
            let bucket_index = self.get_bucket_index(distance);
            let bucket = &mut self.buckets[bucket_index];
            
            // Check if node already exists in bucket
            let existing_index = bucket.iter().position(|n| n.id == node.id);
            
            if let Some(index) = existing_index {
                // Update existing node
                bucket[index] = node;
            } else if bucket.len() < 20 { // K=20 for k-buckets
                // Add new node
                bucket.push(node);
            } else {
                // Bucket is full, in a real implementation we would ping the least recently seen node
                // and replace it if it doesn't respond
                // For now, just replace the least recently seen
                if let Some(min_index) = bucket.iter()
                    .enumerate()
                    .min_by_key(|(_, n)| n.last_seen)
                    .map(|(i, _)| i)
                {
                    bucket[min_index] = node;
                }
            }
            
            debug!("Added node to DHT: {}", node.id);
            Ok(())
        } else {
            Err(NetworkError::Other("DHT not initialized".into()))
        }
    }
    
    /// Find nodes close to a given key
    pub fn find_nodes(&self, key: &Key) -> Result<Vec<NodeInfo>> {
        // In a real implementation, this would perform a Kademlia lookup
        // For now, just return all nodes from the closest bucket
        
        if self.local_address.is_none() {
            return Err(NetworkError::Other("DHT not initialized".into()));
        }
        
        // Calculate bucket index for key
        let bucket_index = self.get_key_bucket_index(key);
        
        // Start with nodes in the closest bucket
        let mut closest_nodes: Vec<NodeInfo> = self.buckets[bucket_index].clone();
        
        // If we need more nodes, look in adjacent buckets
        let mut distance = 1;
        while closest_nodes.len() < 20 && (bucket_index as i32 - distance >= 0 || bucket_index + distance < self.buckets.len()) {
            if bucket_index as i32 - distance >= 0 {
                closest_nodes.extend_from_slice(&self.buckets[bucket_index - distance]);
            }
            
            if bucket_index + distance < self.buckets.len() {
                closest_nodes.extend_from_slice(&self.buckets[bucket_index + distance]);
            }
            
            distance += 1;
        }
        
        // Limit to 20 nodes
        closest_nodes.truncate(20);
        
        Ok(closest_nodes)
    }
    
    /// Calculate XOR distance between two addresses
    fn calculate_distance(&self, addr1: &OverlayAddress, addr2: &OverlayAddress) -> [u8; 16] {
        let mut distance = [0u8; 16];
        
        for i in 0..16 {
            distance[i] = addr1.bytes[i] ^ addr2.bytes[i];
        }
        
        distance
    }
    
    /// Get the bucket index for a distance
    fn get_bucket_index(&self, distance: [u8; 16]) -> usize {
        // Find the index of the first non-zero bit
        for i in 0..16 {
            let byte = distance[i];
            if byte != 0 {
                // Find the position of the first set bit
                for j in 0..8 {
                    if (byte & (1 << (7 - j))) != 0 {
                        return i * 8 + j;
                    }
                }
            }
        }
        
        // All bits are 0, use the last bucket
        255
    }
    
    /// Get the bucket index for a key
    fn get_key_bucket_index(&self, key: &Key) -> usize {
        // Hash the key to get a value we can use for bucket selection
        use sha2::{Sha256, Digest};
        
        let mut hasher = Sha256::new();
        hasher.update(key);
        let hash = hasher.finalize();
        
        // Use the first byte as the bucket index
        hash[0] as usize % self.buckets.len()
    }
}
