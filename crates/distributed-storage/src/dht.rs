use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{RwLock, Mutex};
use thiserror::Error;
use sha2::{Sha256, Digest};
use uuid::Uuid;

/// Errors that can occur in DHT operations
#[derive(Debug, Error)]
pub enum DhtError {
    #[error("Key not found")]
    NotFound,
    
    #[error("IO error: {0}")]
    IoError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Timeout error: {0}")]
    Timeout(String),
    
    #[error("Invalid node ID: {0}")]
    InvalidNodeId(String),
    
    #[error("Routing table full")]
    RoutingTableFull,
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Result type for DHT operations
pub type DhtResult<T> = Result<T, DhtError>;

/// Node ID type (32 bytes)
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct NodeId([u8; 32]);

impl NodeId {
    /// Generate a new random node ID
    pub fn new() -> Self {
        let mut id = [0u8; 32];
        let uuid = Uuid::new_v4();
        id.copy_from_slice(&uuid.as_bytes());
        Self(id)
    }
    
    /// Create a node ID from bytes
    pub fn from_bytes(bytes: &[u8]) -> DhtResult<Self> {
        if bytes.len() != 32 {
            return Err(DhtError::InvalidNodeId("Invalid length".to_string()));
        }
        let mut id = [0u8; 32];
        id.copy_from_slice(bytes);
        Ok(Self(id))
    }
    
    /// Get the XOR distance between two node IDs
    pub fn distance(&self, other: &NodeId) -> u32 {
        let mut distance = 0u32;
        for i in 0..32 {
            distance = (distance << 8) | (self.0[i] ^ other.0[i]) as u32;
        }
        distance
    }
    
    /// Get the bytes of the node ID
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// Information about a DHT node
#[derive(Debug, Clone)]
pub struct NodeInfo {
    /// Node's ID
    pub id: NodeId,
    /// Node's network address
    pub address: String,
    /// Last time we heard from this node
    pub last_seen: Instant,
    /// Whether this node is active
    pub is_active: bool,
}

/// A bucket in the routing table
#[derive(Debug)]
struct Bucket {
    /// Nodes in this bucket
    nodes: Vec<NodeInfo>,
    /// Last time this bucket was updated
    last_updated: Instant,
}

impl Bucket {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            last_updated: Instant::now(),
        }
    }
    
    fn add_node(&mut self, node: NodeInfo) -> bool {
        if self.nodes.len() >= 20 { // Kademlia k=20
            return false;
        }
        if let Some(existing) = self.nodes.iter_mut().find(|n| n.id == node.id) {
            existing.last_seen = node.last_seen;
            existing.is_active = true;
        } else {
            self.nodes.push(node);
        }
        self.last_updated = Instant::now();
        true
    }
    
    fn remove_node(&mut self, node_id: &NodeId) {
        self.nodes.retain(|n| n.id != *node_id);
    }
    
    fn get_active_nodes(&self) -> Vec<NodeInfo> {
        self.nodes.iter()
            .filter(|n| n.is_active)
            .cloned()
            .collect()
    }
}

/// A key-value pair in the DHT
#[derive(Debug, Clone)]
struct KeyValue {
    /// The key
    key: Vec<u8>,
    /// The value
    value: Vec<u8>,
    /// When this entry was last updated
    last_updated: Instant,
    /// Number of replicas
    replica_count: u8,
}

/// The distributed hash table implementation
pub struct DistributedHashTable {
    /// Our node ID
    node_id: NodeId,
    /// Our network address
    address: String,
    /// The routing table (160 buckets for 160-bit IDs)
    routing_table: Vec<Bucket>,
    /// Local key-value store
    storage: HashMap<Vec<u8>, KeyValue>,
    /// Set of nodes we've seen recently
    seen_nodes: HashSet<NodeId>,
    /// Lock for thread-safe access
    lock: RwLock<()>,
}

impl DistributedHashTable {
    /// Create a new DHT instance
    pub fn new(address: String) -> Self {
        Self {
            node_id: NodeId::new(),
            address,
            routing_table: vec![Bucket::new(); 160],
            storage: HashMap::new(),
            seen_nodes: HashSet::new(),
            lock: RwLock::new(()),
        }
    }
    
    /// Get the bucket index for a node ID
    fn get_bucket_index(&self, node_id: &NodeId) -> usize {
        let distance = self.node_id.distance(node_id);
        let leading_zeros = distance.leading_zeros();
        (160 - leading_zeros).min(159) as usize
    }
    
    /// Add a node to the routing table
    async fn add_node(&self, node: NodeInfo) -> bool {
        let _guard = self.lock.write().await;
        let bucket_index = self.get_bucket_index(&node.id);
        self.routing_table[bucket_index].add_node(node)
    }
    
    /// Find the k closest nodes to a given node ID
    async fn find_closest_nodes(&self, target_id: &NodeId, k: usize) -> Vec<NodeInfo> {
        let _guard = self.lock.read().await;
        let mut all_nodes = Vec::new();
        
        // Collect all active nodes
        for bucket in &self.routing_table {
            all_nodes.extend(bucket.get_active_nodes());
        }
        
        // Sort by distance to target
        all_nodes.sort_by(|a, b| {
            a.id.distance(target_id).cmp(&b.id.distance(target_id))
        });
        
        // Take the k closest nodes
        all_nodes.into_iter().take(k).collect()
    }
    
    /// Store data in the DHT
    pub async fn store(&self, key: Vec<u8>, value: Vec<u8>) -> DhtResult<()> {
        let _guard = self.lock.write().await;
        
        // Create key hash for DHT routing
        let mut hasher = Sha256::new();
        hasher.update(&key);
        let key_hash = NodeId::from_bytes(&hasher.finalize())?;
        
        // Find closest nodes
        let closest_nodes = self.find_closest_nodes(&key_hash, 3).await;
        
        // Store locally
        let kv = KeyValue {
            key: key.clone(),
            value: value.clone(),
            last_updated: Instant::now(),
            replica_count: 1,
        };
        self.storage.insert(key, kv);
        
        // TODO: Replicate to closest nodes
        Ok(())
    }
    
    /// Retrieve data from the DHT
    pub async fn get(&self, key: &[u8]) -> DhtResult<Vec<u8>> {
        let _guard = self.lock.read().await;
        
        // Create key hash for DHT routing
        let mut hasher = Sha256::new();
        hasher.update(key);
        let key_hash = NodeId::from_bytes(&hasher.finalize())?;
        
        // Check local storage first
        if let Some(kv) = self.storage.get(key) {
            return Ok(kv.value.clone());
        }
        
        // TODO: Query closest nodes
        Err(DhtError::NotFound)
    }
    
    /// Check if a key exists in the DHT
    pub async fn exists(&self, key: &[u8]) -> DhtResult<bool> {
        let _guard = self.lock.read().await;
        Ok(self.storage.contains_key(key))
    }
    
    /// Remove data from the DHT
    pub async fn remove(&self, key: &[u8]) -> DhtResult<()> {
        let _guard = self.lock.write().await;
        
        if self.storage.remove(key).is_none() {
            return Err(DhtError::NotFound);
        }
        
        // TODO: Remove from replicas
        Ok(())
    }
    
    /// List keys with a given prefix
    pub async fn list_keys(&self, prefix: &[u8]) -> DhtResult<Vec<Vec<u8>>> {
        let _guard = self.lock.read().await;
        Ok(self.storage.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect())
    }
}

impl Default for DistributedHashTable {
    fn default() -> Self {
        Self::new("127.0.0.1:8000".to_string())
    }
}

// Implement the same methods for Arc<DistributedHashTable>
impl<T: std::ops::Deref<Target = DistributedHashTable> + Send + Sync> Arc<T> {
    pub async fn store(&self, key: Vec<u8>, value: Vec<u8>) -> DhtResult<()> {
        self.deref().store(key, value).await
    }
    
    pub async fn get(&self, key: &[u8]) -> DhtResult<Vec<u8>> {
        self.deref().get(key).await
    }
    
    pub async fn exists(&self, key: &[u8]) -> DhtResult<bool> {
        self.deref().exists(key).await
    }
    
    pub async fn remove(&self, key: &[u8]) -> DhtResult<()> {
        self.deref().remove(key).await
    }
    
    pub async fn list_keys(&self, prefix: &[u8]) -> DhtResult<Vec<Vec<u8>>> {
        self.deref().list_keys(prefix).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_node_id() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        
        assert_eq!(id1.as_bytes().len(), 32);
        assert_eq!(id2.as_bytes().len(), 32);
        assert_ne!(id1, id2);
        
        let distance = id1.distance(&id2);
        assert!(distance > 0);
    }
    
    #[tokio::test]
    async fn test_dht_operations() {
        let dht = DistributedHashTable::new("127.0.0.1:8000".to_string());
        
        // Test store and get
        let key = vec![1, 2, 3];
        let value = vec![4, 5, 6];
        
        dht.store(key.clone(), value.clone()).await.unwrap();
        let retrieved = dht.get(&key).await.unwrap();
        assert_eq!(retrieved, value);
        
        // Test exists
        assert!(dht.exists(&key).await.unwrap());
        assert!(!dht.exists(&[9, 9, 9]).await.unwrap());
        
        // Test remove
        dht.remove(&key).await.unwrap();
        assert!(!dht.exists(&key).await.unwrap());
        
        // Test list_keys
        dht.store(vec![1, 2, 3], vec![1]).await.unwrap();
        dht.store(vec![1, 2, 4], vec![2]).await.unwrap();
        dht.store(vec![2, 3, 4], vec![3]).await.unwrap();
        
        let keys = dht.list_keys(&[1, 2]).await.unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&vec![1, 2, 3]));
        assert!(keys.contains(&vec![1, 2, 4]));
    }
    
    #[tokio::test]
    async fn test_arc_dht() {
        let dht = Arc::new(DistributedHashTable::new("127.0.0.1:8000".to_string()));
        
        let key = vec![1, 2, 3];
        let value = vec![4, 5, 6];
        
        dht.store(key.clone(), value.clone()).await.unwrap();
        let retrieved = dht.get(&key).await.unwrap();
        assert_eq!(retrieved, value);
    }
} 