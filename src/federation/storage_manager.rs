use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

use crate::distributed_storage::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType};
use crate::federation_storage_router::{FederationStorageRouter, StorageRoute};
use crate::federation::coordination::FederationCoordinator;
use crate::networking::overlay::dht::DistributedHashTable;
use crate::storage::{Storage, StorageError};

// Federation storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStorageConfig {
    pub federation_id: String,
    pub max_storage_percentage: f32,
    pub auto_replication: bool,
    pub default_redundancy_factor: u8,
    pub enable_cross_federation_storage: bool,
    pub storage_namespace: String,
}

impl Default for FederationStorageConfig {
    fn default() -> Self {
        Self {
            federation_id: "default".to_string(),
            max_storage_percentage: 0.8,
            auto_replication: true,
            default_redundancy_factor: 3,
            enable_cross_federation_storage: true,
            storage_namespace: "federation-data".to_string(),
        }
    }
}

// Federation storage manager
pub struct FederationStorageManager {
    // Configuration
    config: FederationStorageConfig,
    // Distributed storage for this federation
    distributed_storage: Arc<DistributedStorage>,
    // Cross-federation storage router
    federation_router: Arc<FederationStorageRouter>,
    // Federation coordinator for member and agreement management
    federation_coordinator: Arc<FederationCoordinator>,
    // Local storage peers in this federation
    local_peers: RwLock<HashMap<String, StoragePeer>>,
    // Storage peer health metrics
    peer_health: RwLock<HashMap<String, f32>>,
    local_storage: Arc<dyn Storage>,
}

impl FederationStorageManager {
    // Create a new federation storage manager
    pub fn new(
        config: FederationStorageConfig,
        local_storage: Arc<dyn Storage>,
        dht: Arc<DistributedHashTable>,
        federation_coordinator: Arc<FederationCoordinator>,
        node_id: String,
    ) -> Self {
        // Create distributed storage for this federation
        let distributed_storage = Arc::new(DistributedStorage::new(
            node_id.clone(),
            config.federation_id.clone(),
            local_storage,
            dht,
            federation_coordinator.clone(),
        ));
        
        // Create federation storage router
        let federation_router = Arc::new(FederationStorageRouter::new(
            config.federation_id.clone(),
            distributed_storage.clone(),
            federation_coordinator.clone(),
        ));
        
        Self {
            config,
            distributed_storage,
            federation_router,
            federation_coordinator,
            local_peers: RwLock::new(HashMap::new()),
            peer_health: RwLock::new(HashMap::new()),
            local_storage,
        }
    }
    
    // Register a local storage peer
    pub async fn register_local_peer(
        &self,
        node_id: String,
        address: String,
        storage_capacity: u64,
        available_space: u64,
        tags: HashMap<String, String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let peer = StoragePeer {
            node_id: node_id.clone(),
            address,
            federation_id: self.config.federation_id.clone(),
            storage_capacity,
            available_space,
            latency_ms: 0, // Local peer has 0 latency
            uptime_percentage: 100.0, // Assume local peer has 100% uptime initially
            tags,
        };
        
        // Add to local peers
        {
            let mut peers = self.local_peers.write().await;
            peers.insert(node_id.clone(), peer.clone());
        }
        
        // Register with distributed storage
        self.distributed_storage.add_peer(peer).await?;
        
        // Initialize health metrics
        {
            let mut health = self.peer_health.write().await;
            health.insert(node_id, 1.0); // Perfect health initially
        }
        
        Ok(())
    }
    
    // Update a local peer's available space
    pub async fn update_peer_space(
        &self,
        node_id: &str,
        available_space: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.local_peers.write().await;
        
        if let Some(peer) = peers.get_mut(node_id) {
            peer.available_space = available_space;
            
            // Re-register with distributed storage to update
            self.distributed_storage.add_peer(peer.clone()).await?;
            
            Ok(())
        } else {
            Err(Box::new(StorageError::SerializationError(
                format!("Peer not found: {}", node_id)
            )))
        }
    }
    
    // Configure cross-federation storage route
    pub async fn configure_federation_route(
        &self,
        key_prefix: String,
        target_federations: Vec<String>,
        priority_order: bool,
        replication_across_federations: bool,
        access_policy: DataAccessPolicy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Create storage route
        let route = StorageRoute {
            key_prefix,
            target_federations,
            priority_order,
            replication_across_federations,
            access_policy,
        };
        
        // Add to federation router
        self.federation_router.add_route(route).await?;
        
        Ok(())
    }
    
    // Store data in the federation (automatically chooses local or remote storage)
    pub async fn store_data(
        &self,
        key: &str,
        data: &[u8],
        policy: Option<DataAccessPolicy>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.federation_router.put(key, data, policy).await
    }
    
    // Retrieve data from the federation
    pub async fn retrieve_data(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        self.federation_router.get(key).await
    }
    
    // Delete data from the federation
    pub async fn delete_data(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.federation_router.delete(key).await
    }
    
    // Create a policy allowing specific federations to access data
    pub async fn create_federation_access_policy(
        &self,
        read_federations: Vec<String>,
        write_federations: Vec<String>,
        admin_federations: Vec<String>,
    ) -> Result<DataAccessPolicy, Box<dyn std::error::Error>> {
        self.federation_router.create_multi_federation_policy(
            read_federations,
            write_federations,
            admin_federations,
            self.config.default_redundancy_factor,
        ).await
    }
    
    // Monitor health of storage peers
    pub async fn update_peer_health(
        &self,
        node_id: &str,
        health_score: f32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut health = self.peer_health.write().await;
        health.insert(node_id.to_string(), health_score);
        
        // If health drops below threshold, we could take remedial action here
        // such as reducing the peer's priority for new storage requests
        
        Ok(())
    }
    
    // Get storage statistics for the federation
    pub async fn get_federation_storage_stats(&self) -> Result<FederationStorageStats, Box<dyn std::error::Error>> {
        let peers = self.local_peers.read().await;
        
        let total_capacity: u64 = peers.values().map(|p| p.storage_capacity).sum();
        let available_space: u64 = peers.values().map(|p| p.available_space).sum();
        let peer_count = peers.len();
        
        Ok(FederationStorageStats {
            federation_id: self.config.federation_id.clone(),
            total_capacity,
            available_space,
            peer_count,
            utilization_percentage: if total_capacity > 0 {
                ((total_capacity - available_space) as f32 / total_capacity as f32) * 100.0
            } else {
                0.0
            },
        })
    }
}

// Federation storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationStorageStats {
    pub federation_id: String,
    pub total_capacity: u64,
    pub available_space: u64,
    pub peer_count: usize,
    pub utilization_percentage: f32,
} 