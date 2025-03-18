use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

use crate::distributed_storage::{DistributedStorage, DataAccessPolicy, AccessType};
use crate::federation::coordination::FederationCoordinator;
use icn_core::storage::StorageError;

// Storage route information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageRoute {
    pub key_prefix: String,
    pub target_federations: Vec<String>,
    pub priority_order: bool,
    pub replication_across_federations: bool,
    pub access_policy: DataAccessPolicy,
}

// Federation storage router handles data routing across multiple federations
pub struct FederationStorageRouter {
    // Local federation's distributed storage
    local_storage: Arc<DistributedStorage>,
    // Federation coordinator for federation info
    federation_coordinator: Arc<FederationCoordinator>,
    // Managed storage routes
    routes: RwLock<Vec<StorageRoute>>,
    // Access to other federation storage systems (federation_id -> storage)
    federation_storage: RwLock<HashMap<String, Arc<DistributedStorage>>>,
    // Local federation ID
    federation_id: String,
}

impl FederationStorageRouter {
    // Create a new federation storage router
    pub fn new(
        federation_id: String,
        local_storage: Arc<DistributedStorage>,
        federation_coordinator: Arc<FederationCoordinator>,
    ) -> Self {
        Self {
            local_storage,
            federation_coordinator,
            routes: RwLock::new(Vec::new()),
            federation_storage: RwLock::new(HashMap::new()),
            federation_id,
        }
    }
    
    // Add a storage route
    pub async fn add_route(&self, route: StorageRoute) -> Result<(), Box<dyn std::error::Error>> {
        let mut routes = self.routes.write().await;
        routes.push(route);
        Ok(())
    }
    
    // Register another federation's storage
    pub async fn register_federation_storage(
        &self,
        federation_id: String,
        storage: Arc<DistributedStorage>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut fed_storage = self.federation_storage.write().await;
        fed_storage.insert(federation_id, storage);
        Ok(())
    }
    
    // Find the appropriate storage system for a key
    async fn get_storage_for_key(
        &self,
        key: &str,
        operation: AccessType,
    ) -> Result<Arc<DistributedStorage>, Box<dyn std::error::Error>> {
        // First, check routes to determine the target federation(s)
        let routes = self.routes.read().await;
        let fed_storage = self.federation_storage.read().await;
        
        // Find matching route
        for route in routes.iter() {
            if key.starts_with(&route.key_prefix) {
                // Check if we have appropriate access rights
                match operation {
                    AccessType::Read => {
                        if !route.access_policy.read_federations.contains(&self.federation_id) {
                            return Err(Box::new(StorageError::PermissionDenied(
                                format!("Federation {} does not have read access for keys with prefix {}", 
                                       self.federation_id, route.key_prefix)
                            )));
                        }
                    },
                    AccessType::Write => {
                        if !route.access_policy.write_federations.contains(&self.federation_id) {
                            return Err(Box::new(StorageError::PermissionDenied(
                                format!("Federation {} does not have write access for keys with prefix {}", 
                                       self.federation_id, route.key_prefix)
                            )));
                        }
                    },
                    AccessType::Admin => {
                        if !route.access_policy.admin_federations.contains(&self.federation_id) {
                            return Err(Box::new(StorageError::PermissionDenied(
                                format!("Federation {} does not have admin access for keys with prefix {}", 
                                       self.federation_id, route.key_prefix)
                            )));
                        }
                    },
                }
                
                // If this key belongs to our federation, use local storage
                if route.target_federations.contains(&self.federation_id) {
                    return Ok(self.local_storage.clone());
                }
                
                // Otherwise, use the first available target federation
                for fed_id in &route.target_federations {
                    if let Some(storage) = fed_storage.get(fed_id) {
                        return Ok(storage.clone());
                    }
                }
                
                // If we get here, we couldn't find any storage for the target federations
                return Err(Box::new(StorageError::SerializationError(
                    format!("No available storage for target federations: {:?}", route.target_federations)
                )));
            }
        }
        
        // If no route matches, use local storage
        Ok(self.local_storage.clone())
    }
    
    // Put data into the appropriate federation storage
    pub async fn put(
        &self,
        key: &str,
        data: &[u8],
        policy: Option<DataAccessPolicy>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let storage = self.get_storage_for_key(key, AccessType::Write).await?;
        
        // Use provided policy or create a default one with local federation access
        let storage_policy = if let Some(p) = policy {
            p
        } else {
            let mut default_policy = DataAccessPolicy::default();
            default_policy.read_federations.insert(self.federation_id.clone());
            default_policy.write_federations.insert(self.federation_id.clone());
            default_policy.admin_federations.insert(self.federation_id.clone());
            default_policy
        };
        
        storage.put(key, data, storage_policy).await?;
        Ok(())
    }
    
    // Get data from the appropriate federation storage
    pub async fn get(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let storage = self.get_storage_for_key(key, AccessType::Read).await?;
        let data = storage.get(key).await?;
        Ok(data)
    }
    
    // Delete data from the appropriate federation storage
    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        let storage = self.get_storage_for_key(key, AccessType::Admin).await?;
        storage.delete(key).await?;
        Ok(())
    }
    
    // Check if we have access to a key
    pub async fn check_access(
        &self,
        key: &str,
        operation: AccessType,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // First try to get the appropriate storage (this checks route permissions)
        match self.get_storage_for_key(key, operation).await {
            Ok(storage) => {
                // Then check if we have access at the storage level
                storage.check_access(key, operation).await
            },
            Err(_) => {
                // If we can't get storage, we don't have access
                Ok(false)
            }
        }
    }
    
    // Create a multi-federation data access policy
    pub async fn create_multi_federation_policy(
        &self,
        read_federations: Vec<String>,
        write_federations: Vec<String>,
        admin_federations: Vec<String>,
        redundancy_factor: u8,
    ) -> Result<DataAccessPolicy, Box<dyn std::error::Error>> {
        // Verify that federations exist
        let all_federations: HashSet<String> = read_federations.iter()
            .chain(write_federations.iter())
            .chain(admin_federations.iter())
            .cloned()
            .collect();
        
        // In a real implementation, verify federations exist via federation_coordinator
        
        let mut policy = DataAccessPolicy::default();
        policy.read_federations = read_federations.into_iter().collect();
        policy.write_federations = write_federations.into_iter().collect();
        policy.admin_federations = admin_federations.into_iter().collect();
        policy.redundancy_factor = redundancy_factor;
        
        Ok(policy)
    }
} 