use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use serde::{Serialize, Deserialize};
use crate::error::Result;
use crate::overlay::address::OverlayAddress;

/// DHT key
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Key(Vec<u8>);

impl Key {
    /// Create a new key
    pub fn new(data: Vec<u8>) -> Self {
        Key(data)
    }
    
    /// Create a key from a string
    pub fn from_string(s: &str) -> Self {
        Key(s.as_bytes().to_vec())
    }
    
    /// Get the byte representation
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// DHT value
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value(Vec<u8>);

impl Value {
    /// Create a new value
    pub fn new(data: Vec<u8>) -> Self {
        Value(data)
    }
    
    /// Create a value from a string
    pub fn from_string(s: &str) -> Self {
        Value(s.as_bytes().to_vec())
    }
    
    /// Get the byte representation
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Node information for DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Node ID
    pub id: String,
    /// Overlay address
    pub address: OverlayAddress,
    /// Last seen timestamp
    pub last_seen: u64,
    /// Additional information
    pub metadata: HashMap<String, String>,
}

/// Distributed Hash Table for the overlay network
pub struct DistributedHashTable {
    /// Local node ID
    node_id: String,
    /// Local overlay address
    address: Option<OverlayAddress>,
    /// Known peers
    peers: Arc<RwLock<HashMap<String, NodeInfo>>>,
    /// Stored key-value pairs
    storage: Arc<RwLock<HashMap<Key, Value>>>,
}

impl DistributedHashTable {
    /// Create a new DHT
    pub fn new() -> Self {
        Self {
            node_id: String::new(),
            address: None,
            peers: Arc::new(RwLock::new(HashMap::new())),
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Initialize the DHT
    pub fn initialize(&mut self, node_id: String, address: OverlayAddress) {
        self.node_id = node_id;
        self.address = Some(address);
    }
    
    /// Store a key-value pair
    pub async fn put(&self, key: Key, value: Value) -> Result<()> {
        let mut storage = self.storage.write().unwrap();
        storage.insert(key, value);
        Ok(())
    }
    
    /// Retrieve a value
    pub async fn get(&self, key: &Key) -> Result<Option<Value>> {
        let storage = self.storage.read().unwrap();
        Ok(storage.get(key).cloned())
    }
    
    /// Remove a key-value pair
    pub async fn delete(&self, key: &Key) -> Result<()> {
        let mut storage = self.storage.write().unwrap();
        storage.remove(key);
        Ok(())
    }
    
    /// Add a peer to the DHT
    pub async fn add_peer(&self, info: NodeInfo) -> Result<()> {
        let mut peers = self.peers.write().unwrap();
        peers.insert(info.id.clone(), info);
        Ok(())
    }
    
    /// Get a list of peers
    pub async fn get_peers(&self) -> Result<Vec<NodeInfo>> {
        let peers = self.peers.read().unwrap();
        Ok(peers.values().cloned().collect())
    }
    
    /// Find the closest peers to a key
    pub async fn find_closest_peers(&self, key: &Key, count: usize) -> Result<Vec<NodeInfo>> {
        let peers = self.peers.read().unwrap();
        // In a real implementation, this would calculate XOR distance
        // For now, just return up to 'count' peers
        Ok(peers.values().cloned().take(count).collect())
    }
} 