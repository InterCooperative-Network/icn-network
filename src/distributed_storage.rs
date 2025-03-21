use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use serde::{Deserialize, Serialize};

use crate::federation::coordination::{FederationCoordinator, SharedResource};
use icn_core::storage::{Storage, StorageError, StorageResult};
use icn_core::storage::versioning::{VersionInfo as CoreVersionInfo, VersionHistory as CoreVersionHistory, VersioningManager as CoreVersioningManager, VersioningError as CoreVersioningError};
use icn_core::networking::overlay::dht::DistributedHashTable;
use crate::crypto::{StorageEncryptionService as ImportedStorageEncryptionService, EncryptionMetadata as ImportedEncryptionMetadata, EncryptionError as ImportedEncryptionError};
use icn_core::storage::{
    QuotaManager, OperationScheduler, QuotaOperation
};

// Add missing enum for quota checking
#[derive(Debug, Clone, PartialEq)]
pub enum QuotaCheckResult {
    Allowed,
    Throttled { reason: String, retry_after_secs: u64 },
    Denied { reason: String },
}

// Now using our own version of these types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub version_id: String,
    pub created_at: u64,
    pub size_bytes: u64,
    pub metadata: HashMap<String, String>,
    pub storage_key: String,
    pub content_hash: String,
    pub created_by: String,
    pub comment: Option<String>,
}

#[derive(Clone)]
pub struct VersionHistory {
    pub key: String,
    pub versions: Vec<VersionInfo>,
    pub max_versions: u32,
    // Add missing fields
    pub current_version_id: String,
    pub total_size_bytes: u64,
}

pub struct VersioningManager {}

#[derive(Debug, thiserror::Error)]
pub enum VersioningError {
    #[error("Version not found: {0}")]
    VersionNotFound(String),
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    #[error("Other error: {0}")]
    Other(String),
}

// Placeholder for networking module
pub mod networking {
    pub mod overlay {
        pub mod dht {
            use std::sync::Arc;
            use std::io::Result;
            
            #[derive(Debug, thiserror::Error)]
            pub enum DhtError {
                #[error("Not found")]
                NotFound,
                #[error("IO error: {0}")]
                IoError(String),
                #[error("Other error: {0}")]
                Other(String),
            }
            
            pub type DhtResult<T> = std::result::Result<T, DhtError>;
            
            pub struct DistributedHashTable {}
            
            impl DistributedHashTable {
                pub fn new() -> Self {
                    Self {}
                }
                
                pub fn get(&self, key: &Vec<u8>) -> DhtResult<Vec<u8>> {
                    // Placeholder implementation for DHT get
                    Err(DhtError::NotFound)
                }
                
                pub fn store(&self, key: Vec<u8>, value: Vec<u8>) -> DhtResult<()> {
                    // Placeholder implementation for DHT store
                    Ok(())
                }
            }
            
            // Implement the same methods for Arc<DistributedHashTable>
            impl<T: std::ops::Deref<Target = DistributedHashTable> + Send + Sync> Arc<T> {
                pub fn get(&self, key: &Vec<u8>) -> DhtResult<Vec<u8>> {
                    self.deref().get(key)
                }
                
                pub fn store(&self, key: Vec<u8>, value: Vec<u8>) -> DhtResult<()> {
                    self.deref().store(key, value)
                }
            }
        }
    }
}

// Placeholder for crypto services
pub struct StorageEncryptionService {}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionMetadata {
    pub key_id: String,
    pub iv: Vec<u8>,
    pub tag: Vec<u8>,
    pub encryption_type: String,
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    #[error("Key management error: {0}")]
    KeyManagementError(String),
    #[error("Key not found: {0}")]
    KeyNotFound(String),
    #[error("Access denied: {0}")]
    AccessDenied(String),
}

// Add placeholders for quota management
pub struct QuotaManager {}

impl QuotaManager {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn check_quota(&self, entity_id: &str, operation: &QuotaOperation) -> QuotaCheckResult {
        // In a placeholder implementation, always allow
        QuotaCheckResult::Allowed
    }
    
    pub async fn update_usage(&self, entity_id: &str, operation: &QuotaOperation) -> Result<(), String> {
        // Update quota usage for an entity
        Ok(())
    }
}

// Implement methods for Arc<QuotaManager>
impl<T: std::ops::Deref<Target = QuotaManager> + Send + Sync> std::sync::Arc<T> {
    pub async fn check_quota(&self, entity_id: &str, operation: &QuotaOperation) -> QuotaCheckResult {
        self.deref().check_quota(entity_id, operation).await
    }
    
    pub async fn update_usage(&self, entity_id: &str, operation: &QuotaOperation) -> Result<(), String> {
        self.deref().update_usage(entity_id, operation).await
    }
}

pub struct OperationScheduler {}

impl OperationScheduler {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn start(&self) -> Result<(), String> {
        // Start the scheduler
        Ok(())
    }
    
    pub async fn can_execute_immediately(&self, entity_id: &str, operation: &QuotaOperation) -> bool {
        // Check if an operation can be executed immediately
        true
    }
}

// Implement methods for Arc<OperationScheduler>
impl<T: std::ops::Deref<Target = OperationScheduler> + Send + Sync> std::sync::Arc<T> {
    pub async fn start(&self) -> Result<(), String> {
        self.deref().start().await
    }
    
    pub async fn can_execute_immediately(&self, entity_id: &str, operation: &QuotaOperation) -> bool {
        self.deref().can_execute_immediately(entity_id, operation).await
    }
}

// Use the placeholders instead of importing them
use networking::overlay::dht::DistributedHashTable;

// Storage peer information with proximity scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePeer {
    pub node_id: String,
    pub address: String,
    pub federation_id: String,
    pub storage_capacity: u64,
    pub available_space: u64,
    pub latency_ms: u32,
    pub uptime_percentage: f32,
    pub tags: HashMap<String, String>,
}

// Access policy for data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataAccessPolicy {
    pub read_federations: HashSet<String>,
    pub write_federations: HashSet<String>,
    pub admin_federations: HashSet<String>,
    pub encryption_required: bool,
    pub redundancy_factor: u8,
    pub expiration_time: Option<u64>,
    pub versioning_enabled: bool,
    pub max_versions: u32,
}

impl Default for DataAccessPolicy {
    fn default() -> Self {
        Self {
            read_federations: HashSet::new(),
            write_federations: HashSet::new(),
            admin_federations: HashSet::new(),
            encryption_required: true,
            redundancy_factor: 3,
            expiration_time: None,
            versioning_enabled: false,
            max_versions: 10,
        }
    }
}

// Data location tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataLocation {
    pub key: String,
    pub storage_peers: Vec<String>,
    pub policy: DataAccessPolicy,
    pub content_hash: String,
    pub size_bytes: u64,
    pub created_at: u64,
    pub updated_at: u64,
    // Encryption metadata if encrypted
    pub encryption_metadata: Option<EncryptionMetadata>,
    // Versioning metadata if versioning is enabled
    pub version_info: Option<VersionInfo>,
    // Whether this is a versioned object
    pub is_versioned: bool,
}

// Distributed storage system
pub struct DistributedStorage {
    // Local storage for this node
    local_storage: Arc<dyn Storage>,
    // DHT for distributed lookups
    dht: Arc<DistributedHashTable>,
    // Federation coordinator for access control
    federation_coordinator: Arc<FederationCoordinator>,
    // Encryption service for data encryption
    encryption_service: Arc<StorageEncryptionService>,
    // Versioning manager for data versioning
    versioning_manager: Arc<VersioningManager>,
    // Cache of known storage peers
    peers: RwLock<HashMap<String, StoragePeer>>,
    // Cache of data locations
    data_locations: RwLock<HashMap<String, DataLocation>>,
    // Local node information
    node_id: String,
    federation_id: String,
    /// Quota manager for resource allocation
    pub quota_manager: Option<Arc<QuotaManager>>,
    /// Operation scheduler for priority-based processing
    pub operation_scheduler: Option<Arc<OperationScheduler>>,
}

impl DistributedStorage {
    // Create a new distributed storage instance
    pub fn new(
        node_id: String,
        federation_id: String,
        local_storage: Arc<dyn Storage>,
        dht: Arc<DistributedHashTable>,
        federation_coordinator: Arc<FederationCoordinator>,
    ) -> Self {
        let encryption_service = Arc::new(StorageEncryptionService::new());
        let versioning_manager = Arc::new(VersioningManager::new());
        
        Self {
            local_storage,
            dht,
            federation_coordinator,
            encryption_service,
            versioning_manager,
            peers: RwLock::new(HashMap::new()),
            data_locations: RwLock::new(HashMap::new()),
            node_id,
            federation_id,
            quota_manager: None,
            operation_scheduler: None,
        }
    }
    
    // Create with a custom encryption service
    pub fn with_encryption_service(
        node_id: String,
        federation_id: String,
        local_storage: Arc<dyn Storage>,
        dht: Arc<DistributedHashTable>,
        federation_coordinator: Arc<FederationCoordinator>,
        encryption_service: Arc<StorageEncryptionService>,
    ) -> Self {
        let versioning_manager = Arc::new(VersioningManager::new());
        
        Self {
            local_storage,
            dht,
            federation_coordinator,
            encryption_service,
            versioning_manager,
            peers: RwLock::new(HashMap::new()),
            data_locations: RwLock::new(HashMap::new()),
            node_id,
            federation_id,
            quota_manager: None,
            operation_scheduler: None,
        }
    }
    
    // Add a storage peer to the known peers
    pub async fn add_peer(&self, peer: StoragePeer) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.peers.write().await;
        peers.insert(peer.node_id.clone(), peer);
        Ok(())
    }
    
    // Remove a storage peer
    pub async fn remove_peer(&self, node_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut peers = self.peers.write().await;
        peers.remove(node_id);
        Ok(())
    }
    
    // Find the best storage peers for storing data based on proximity and capacity
    pub async fn select_storage_peers(
        &self,
        redundancy_factor: u8,
        min_available_space: u64,
        preferred_federation_id: Option<String>,
    ) -> Result<Vec<StoragePeer>, Box<dyn std::error::Error>> {
        let peers = self.peers.read().await;
        
        // Filter out peers without enough space
        let eligible_peers: Vec<StoragePeer> = peers.values()
            .filter(|p| p.available_space >= min_available_space)
            .cloned()
            .collect();
        
        if eligible_peers.len() < redundancy_factor as usize {
            return Err(Box::new(StorageError::InsufficientResources(
                format!("Not enough peers with required capacity. Need: {}, Found: {}", 
                       redundancy_factor, eligible_peers.len())
            )));
        }
        
        // Score peers by proximity (lower latency is better) and prefer the specified federation
        let mut scored_peers: Vec<(f32, StoragePeer)> = eligible_peers.into_iter()
            .map(|peer| {
                let federation_score = if let Some(ref pref_fed) = preferred_federation_id {
                    if peer.federation_id == *pref_fed { 0.0 } else { 100.0 }
                } else {
                    0.0
                };
                
                // Combined score (lower is better)
                let score = peer.latency_ms as f32 + federation_score - (peer.uptime_percentage / 10.0);
                
                (score, peer)
            })
            .collect();
        
        // Sort by score (lower is better)
        scored_peers.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Take the top N peers
        let selected_peers = scored_peers.into_iter()
            .take(redundancy_factor as usize)
            .map(|(_, peer)| peer)
            .collect();
        
        Ok(selected_peers)
    }
    
    // Check if the current node has access to data
    pub async fn check_access(
        &self,
        data_key: &str,
        access_type: AccessType,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Get data location from DHT or cache
        let data_location = self.get_data_location(data_key).await?;
        
        // Check if the current federation has appropriate access
        match access_type {
            AccessType::Read => {
                Ok(data_location.policy.read_federations.contains(&self.federation_id))
            },
            AccessType::Write => {
                Ok(data_location.policy.write_federations.contains(&self.federation_id))
            },
            AccessType::Admin => {
                Ok(data_location.policy.admin_federations.contains(&self.federation_id))
            },
        }
    }
    
    // Initialize encryption key for a federation
    pub async fn initialize_encryption_key(
        &self, 
        federations: Vec<String>
    ) -> Result<String, Box<dyn std::error::Error>> {
        self.encryption_service.generate_key(federations).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    
    // Grant federation access to an encryption key
    pub async fn grant_federation_key_access(
        &self,
        federation_id: &str,
        key_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.encryption_service.grant_federation_key_access(federation_id, key_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
    }
    
    // Store data with distributed redundancy
    pub async fn put(
        &self,
        key: &str,
        data: &[u8],
        policy: DataAccessPolicy,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Check quota for the current federation
        if let Some(quota_manager) = &self.quota_manager {
            let quota_check = self.check_quota(
                &self.federation_id,
                QuotaOperation::Put { size_bytes: data.len() as u64 },
            ).await;
            
            if let Err(result) = quota_check {
                match result {
                    QuotaCheckResult::Throttled { reason, retry_after_secs } => {
                        return Err(format!("Operation throttled: {}. Retry after {} seconds.", 
                            reason, retry_after_secs).into());
                    },
                    QuotaCheckResult::Denied { reason } => {
                        return Err(format!("Operation denied: {}", reason).into());
                    },
                    _ => {} // Should not happen
                }
            }
        }
        
        // Check if we have write access for this key
        if !policy.write_federations.contains(&self.federation_id) {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have write access", self.federation_id)
            )));
        }
        
        // Select storage peers based on the redundancy factor in policy
        let peers = self.select_storage_peers(
            policy.redundancy_factor,
            data.len() as u64,
            Some(self.federation_id.clone()),
        ).await?;
        
        // Prepare the data for storage (encrypt if required)
        let (storage_data, encryption_metadata) = if policy.encryption_required {
            // Create a key if we don't have one yet
            let mut federations = policy.read_federations.clone();
            federations.extend(policy.write_federations.clone());
            federations.extend(policy.admin_federations.clone());
            
            let federations_vec: Vec<String> = federations.into_iter().collect();
            
            // Generate a key or use an existing one
            let key_id = match self.encryption_service.generate_key(federations_vec).await {
                Ok(id) => id,
                Err(e) => return Err(Box::new(e)),
            };
            
            // Encrypt the data
            let (encrypted_data, metadata) = self.encryption_service.encrypt(
                data,
                Some(&key_id),
                Some(key.as_bytes()),
            ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
            
            (encrypted_data, Some(metadata))
        } else {
            (data.to_vec(), None)
        };
        
        // Handle versioning if enabled
        let version_info = if policy.versioning_enabled {
            // Check if this key already has versions
            let existing_location = self.get_data_location(key).await.ok();
            
            if let Some(location) = &existing_location {
                if location.is_versioned {
                    // Create a new version for existing data
                    let version_id = self.versioning_manager.generate_version_id();
                    let version_storage_key = self.versioning_manager.create_version_storage_key(key, &version_id);
                    
                    // Calculate content hash for integrity verification
                    let content_hash = compute_hash(data);
                    
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs();
                    
                    // Create version info
                    let version = VersionInfo {
                        version_id: version_id.clone(),
                        storage_key: version_storage_key.clone(),
                        created_at: now,
                        size_bytes: data.len() as u64,
                        content_hash,
                        created_by: self.node_id.clone(),
                        comment: None,
                        metadata: HashMap::new(),
                    };
                    
                    // Store the version data
                    // We store this with a special key to avoid overwriting the main data
                    if peers.iter().any(|p| p.node_id == self.node_id) {
                        self.local_storage.put(&version_storage_key, &storage_data).await?;
                    }
                    
                    // Add to version history
                    self.versioning_manager.create_version(key, &version_id, version.clone()).await
                        .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    
                    Some(version)
                } else {
                    // Key exists but not versioned yet - initialize versioning
                    let version_id = self.versioning_manager.generate_version_id();
                    let version_storage_key = self.versioning_manager.create_version_storage_key(key, &version_id);
                    
                    // Calculate content hash for integrity verification
                    let content_hash = compute_hash(data);
                    
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)?
                        .as_secs();
                    
                    // Create version info
                    let version = VersionInfo {
                        version_id: version_id.clone(),
                        storage_key: version_storage_key.clone(),
                        created_at: now,
                        size_bytes: data.len() as u64,
                        content_hash,
                        created_by: self.node_id.clone(),
                        comment: None,
                        metadata: HashMap::new(),
                    };
                    
                    // Store the version data
                    if peers.iter().any(|p| p.node_id == self.node_id) {
                        self.local_storage.put(&version_storage_key, &storage_data).await?;
                    }
                    
                    // Initialize versioning for this key
                    self.versioning_manager.init_versioning(
                        key,
                        &version_id,
                        version.clone(),
                        policy.max_versions
                    ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    
                    Some(version)
                }
            } else {
                // New data with versioning enabled - initialize versioning
                let version_id = self.versioning_manager.generate_version_id();
                let version_storage_key = self.versioning_manager.create_version_storage_key(key, &version_id);
                
                // Calculate content hash for integrity verification
                let content_hash = compute_hash(data);
                
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                
                // Create version info
                let version = VersionInfo {
                    version_id: version_id.clone(),
                    storage_key: version_storage_key.clone(),
                    created_at: now,
                    size_bytes: data.len() as u64,
                    content_hash,
                    created_by: self.node_id.clone(),
                    comment: None,
                    metadata: HashMap::new(),
                };
                
                // Store the version data and the main data
                if peers.iter().any(|p| p.node_id == self.node_id) {
                    self.local_storage.put(&version_storage_key, &storage_data).await?;
                    self.local_storage.put(key, &storage_data).await?;
                }
                
                // Initialize versioning for this key
                self.versioning_manager.init_versioning(
                    key,
                    &version_id,
                    version.clone(),
                    policy.max_versions
                ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                
                Some(version)
            }
        } else {
            // No versioning - store directly
            if peers.iter().any(|p| p.node_id == self.node_id) {
                self.local_storage.put(key, &storage_data).await?;
            }
            None
        };
        
        // Calculate content hash for integrity verification
        let content_hash = compute_hash(data);
        
        // Create data location entry
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        let data_location = DataLocation {
            key: key.to_string(),
            storage_peers: peers.iter().map(|p| p.node_id.clone()).collect(),
            policy,
            content_hash,
            size_bytes: data.len() as u64,
            created_at: now,
            updated_at: now,
            encryption_metadata,
            version_info: version_info.clone(),
            is_versioned: policy.versioning_enabled,
        };
        
        // Store data location in cache and DHT
        {
            let mut locations = self.data_locations.write().await;
            locations.insert(key.to_string(), data_location.clone());
        }
        
        // Store location metadata in DHT for discovery
        let location_bytes = serde_json::to_vec(&data_location)?;
        self.dht.store(key.as_bytes().to_vec(), location_bytes)?;
        
        // Successfully stored the data, update quota usage
        if let Some(quota_manager) = &self.quota_manager {
            // Update usage statistics
            let _ = quota_manager.update_usage(
                &self.federation_id,
                data.len() as i64,  // Storage increased
                1,                  // Key count increased
                1,                  // One operation
                data.len() as u64,  // Bandwidth used
            ).await;
        }

        Ok(())
    }
    
    // Retrieve data from the distributed storage
    pub async fn get(&self, key: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Check if we have read access for this key
        if !self.check_access(key, AccessType::Read).await? {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have read access", self.federation_id)
            )));
        }
        
        // Get data location to check if encrypted and/or versioned
        let data_location = self.get_data_location(key).await?;
        
        // If versioned, get the current version storage key
        let storage_key = if data_location.is_versioned {
            if let Some(version_info) = &data_location.version_info {
                &version_info.storage_key
            } else {
                // Fallback to the original key if version info is missing
                key
            }
        } else {
            key
        };
        
        // Try local storage first for efficiency
        let storage_result = self.local_storage.get(storage_key).await;
        match storage_result {
            Ok(encrypted_data) => {
                // Decrypt if encrypted
                if let Some(metadata) = &data_location.encryption_metadata {
                    // Check if we have access to the encryption key
                    if !self.encryption_service.federation_has_key_access(&self.federation_id, &metadata.key_id).await {
                        return Err(Box::new(EncryptionError::KeyNotFound(
                            format!("Federation does not have access to encryption key: {}", metadata.key_id)
                        )));
                    }
                    
                    // Decrypt the data
                    let decrypted = self.encryption_service.decrypt(
                        &encrypted_data,
                        metadata,
                        Some(key.as_bytes()),
                    ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    
                    return Ok(decrypted);
                }
                
                return Ok(encrypted_data);
            },
            Err(_) => {
                // Not available locally, find it in the distributed system
                // In a real implementation, we would attempt to fetch from multiple storage peers
                if let Some(peer_id) = data_location.storage_peers.first() {
                    if peer_id == &self.node_id {
                        // Should be available locally, but we already checked
                        return Err(Box::new(StorageError::KeyNotFound(key.to_string())));
                    }
                    
                    // In a real implementation, fetch from the peer
                    // For now, simulate a peer fetch failure
                    return Err(Box::new(StorageError::IoError(
                        std::io::Error::new(std::io::ErrorKind::NotFound, "Data not available from peer")
                    )));
                }
                
                return Err(Box::new(StorageError::KeyNotFound(key.to_string())));
            }
        }
    }
    
    // Delete data from the distributed storage
    pub async fn delete(&self, key: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we have admin access for this key
        if !self.check_access(key, AccessType::Admin).await? {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have admin access", self.federation_id)
            )));
        }
        
        // Get data location
        let data_location = self.get_data_location(key).await?;
        
        // Remove from local storage if stored here
        if data_location.storage_peers.contains(&self.node_id) {
            self.local_storage.delete(key).await?;
        }
        
        // Remove from DHT
        // In a real implementation, we would request deletion from all storage peers
        
        // Remove from location cache
        {
            let mut locations = self.data_locations.write().await;
            locations.remove(key);
        }
        
        Ok(())
    }
    
    // Get data location from cache or DHT
    async fn get_data_location(&self, key: &str) -> Result<DataLocation, Box<dyn std::error::Error>> {
        // Check cache first
        {
            let locations = self.data_locations.read().await;
            if let Some(location) = locations.get(key) {
                return Ok(location.clone());
            }
        }
        
        // Not in cache, look up in DHT
        let location_bytes = self.dht.get(&key.as_bytes().to_vec())?;
        let data_location: DataLocation = serde_json::from_slice(&location_bytes)?;
        
        // Update cache
        {
            let mut locations = self.data_locations.write().await;
            locations.insert(key.to_string(), data_location.clone());
        }
        
        Ok(data_location)
    }
    
    // Get a specific version of data
    pub async fn get_version(&self, key: &str, version_id: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        // Check if we have read access for this key
        if !self.check_access(key, AccessType::Read).await? {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have read access", self.federation_id)
            )));
        }
        
        // Get data location to check if encrypted and versioned
        let data_location = self.get_data_location(key).await?;
        
        if !data_location.is_versioned {
            return Err(Box::new(VersioningError::KeyNotFound(
                format!("Key is not versioned: {}", key)
            )));
        }
        
        // Get version info
        let version = self.versioning_manager.get_version(key, version_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Try local storage first for efficiency
        let storage_result = self.local_storage.get(&version.storage_key).await;
        match storage_result {
            Ok(encrypted_data) => {
                // Decrypt if encrypted
                if let Some(metadata) = &data_location.encryption_metadata {
                    // Check if we have access to the encryption key
                    if !self.encryption_service.federation_has_key_access(&self.federation_id, &metadata.key_id).await {
                        return Err(Box::new(EncryptionError::KeyNotFound(
                            format!("Federation does not have access to encryption key: {}", metadata.key_id)
                        )));
                    }
                    
                    // Decrypt the data
                    let decrypted = self.encryption_service.decrypt(
                        &encrypted_data,
                        metadata,
                        Some(key.as_bytes()),
                    ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
                    
                    return Ok(decrypted);
                }
                
                return Ok(encrypted_data);
            },
            Err(_) => {
                // Not available locally, would need to find it in the distributed system
                return Err(Box::new(StorageError::KeyNotFound(format!("Version {} of key {} not found locally", version_id, key))));
            }
        }
    }
    
    // List all versions for a key
    pub async fn list_versions(&self, key: &str) -> Result<Vec<VersionInfo>, Box<dyn std::error::Error>> {
        // Check if we have read access for this key
        if !self.check_access(key, AccessType::Read).await? {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have read access", self.federation_id)
            )));
        }
        
        // Get data location to check if versioned
        let data_location = self.get_data_location(key).await?;
        
        if !data_location.is_versioned {
            return Err(Box::new(VersioningError::KeyNotFound(
                format!("Key is not versioned: {}", key)
            )));
        }
        
        // Get version history
        let history = self.versioning_manager.get_version_history(key).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Return versions in timeline order
        let versions: Vec<VersionInfo> = history.get_all_versions()
            .into_iter()
            .cloned()
            .collect();
        
        Ok(versions)
    }
    
    // Revert to a specific version
    pub async fn revert_to_version(&self, key: &str, version_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we have write access for this key
        if !self.check_access(key, AccessType::Write).await? {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have write access", self.federation_id)
            )));
        }
        
        // Get data location to check if versioned
        let data_location = self.get_data_location(key).await?;
        
        if !data_location.is_versioned {
            return Err(Box::new(VersioningError::KeyNotFound(
                format!("Key is not versioned: {}", key)
            )));
        }
        
        // Set the specified version as the current version
        self.versioning_manager.set_current_version(key, version_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Get the version info
        let version = self.versioning_manager.get_version(key, version_id).await
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Update the data location to reflect the current version
        let mut updated_location = data_location.clone();
        updated_location.version_info = Some(version);
        updated_location.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Update in cache
        {
            let mut locations = self.data_locations.write().await;
            locations.insert(key.to_string(), updated_location.clone());
        }
        
        // Update in DHT
        let location_bytes = serde_json::to_vec(&updated_location)?;
        self.dht.store(key.as_bytes().to_vec(), location_bytes)?;
        
        Ok(())
    }
    
    // Enable versioning for an existing key
    pub async fn enable_versioning(&self, key: &str, max_versions: u32) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we have admin access for this key
        if !self.check_access(key, AccessType::Admin).await? {
            return Err(Box::new(StorageError::PermissionDenied(
                format!("Current federation {} does not have admin access", self.federation_id)
            )));
        }
        
        // Get data location
        let data_location = self.get_data_location(key).await?;
        
        if data_location.is_versioned {
            // Already versioned
            return Ok(());
        }
        
        // Get the data
        let data = self.get(key).await?;
        
        // Create initial version
        let version_id = self.versioning_manager.generate_version_id();
        let version_storage_key = self.versioning_manager.create_version_storage_key(key, &version_id);
        
        // Calculate content hash for integrity verification
        let content_hash = compute_hash(&data);
        
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Create version info
        let version = VersionInfo {
            version_id: version_id.clone(),
            storage_key: version_storage_key.clone(),
            created_at: now,
            size_bytes: data.len() as u64,
            content_hash,
            created_by: self.node_id.clone(),
            comment: Some("Initial version".to_string()),
            metadata: HashMap::new(),
        };
        
        // Store the version data
        let storage_data = if let Some(metadata) = &data_location.encryption_metadata {
            // Data is already encrypted
            self.local_storage.get(key).await?
        } else {
            data.clone()
        };
        
        // Store in the version storage location
        self.local_storage.put(&version_storage_key, &storage_data).await?;
        
        // Initialize versioning for this key
        self.versioning_manager.init_versioning(
            key,
            &version_id,
            version.clone(),
            max_versions
        ).await.map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
        
        // Update the data location
        let mut updated_location = data_location;
        updated_location.is_versioned = true;
        updated_location.version_info = Some(version);
        updated_location.policy.versioning_enabled = true;
        updated_location.policy.max_versions = max_versions;
        updated_location.updated_at = now;
        
        // Update in cache
        {
            let mut locations = self.data_locations.write().await;
            locations.insert(key.to_string(), updated_location.clone());
        }
        
        // Update in DHT
        let location_bytes = serde_json::to_vec(&updated_location)?;
        self.dht.store(key.as_bytes().to_vec(), location_bytes)?;
        
        Ok(())
    }

    /// Set the quota manager for this storage instance
    pub fn with_quota_manager(mut self, quota_manager: Arc<QuotaManager>) -> Self {
        self.quota_manager = Some(quota_manager.clone());
        self.operation_scheduler = Some(Arc::new(OperationScheduler::new(quota_manager)));
        self
    }

    /// Start the operation scheduler
    pub async fn start_scheduler(&self) -> Result<(), String> {
        if let Some(scheduler) = &self.operation_scheduler {
            scheduler.start().await?;
            Ok(())
        } else {
            Err("No operation scheduler configured".to_string())
        }
    }

    /// Check quota before operating on data
    pub async fn check_quota(
        &self, 
        entity_id: &str, 
        operation: QuotaOperation
    ) -> Result<(), QuotaCheckResult> {
        if let Some(scheduler) = &self.operation_scheduler {
            if !scheduler.can_execute_immediately(entity_id, operation).await {
                // Check the specific reason for denial
                if let Some(quota_manager) = &self.quota_manager {
                    let result = quota_manager.check_quota(entity_id, operation).await;
                    if result != QuotaCheckResult::Allowed {
                        return Err(result);
                    }
                }
            }
        }
        
        Ok(())
    }
}

// Access type enum
#[derive(Debug, Clone, Copy)]
pub enum AccessType {
    Read,
    Write,
    Admin,
}

// Helper function to compute hash of data for integrity verification
fn compute_hash(data: &[u8]) -> String {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:x}", result)
}

// Implement methods for Arc<VersioningManager>
impl std::ops::Deref for VersioningManager {
    type Target = Self;
    
    fn deref(&self) -> &Self::Target {
        self
    }
}

impl VersioningManager {
    pub fn new() -> Self {
        VersioningManager {}
    }
    
    pub async fn generate_version_id(&self) -> String {
        use uuid::Uuid;
        Uuid::new_v4().to_string()
    }
    
    pub fn create_version_storage_key(&self, key: &str, version_id: &str) -> String {
        format!("{}:versions:{}", key, version_id)
    }
    
    pub async fn init_versioning(
        &self,
        key: &str,
        version: VersionInfo,
    ) -> Result<(), VersioningError> {
        let current_version_id = version.version_id.clone();
        
        let history = VersionHistory {
            key: key.to_string(),
            versions: vec![version],
            max_versions: 10,
            current_version_id,
            total_size_bytes: 0,
        };
        
        Ok(())
    }
    
    pub async fn create_version(
        &self,
        key: &str,
        version_id: &str,
        version: VersionInfo,
    ) -> Result<(), VersioningError> {
        Ok(())
    }
    
    pub async fn get_version(
        &self,
        key: &str,
        version_id: &str,
    ) -> Result<VersionInfo, VersioningError> {
        Err(VersioningError::VersionNotFound(version_id.to_string()))
    }
    
    pub async fn get_version_history(&self, key: &str) -> Result<VersionHistory, VersioningError> {
        Ok(VersionHistory {
            key: key.to_string(),
            versions: Vec::new(),
            max_versions: 10,
            current_version_id: String::new(),
            total_size_bytes: 0,
        })
    }
    
    pub async fn set_current_version(&self, key: &str, version_id: &str) -> Result<(), VersioningError> {
        Ok(())
    }
}

// Implement the same methods for Arc<VersioningManager>
impl<T: std::ops::Deref<Target = VersioningManager> + Send + Sync> std::sync::Arc<T> {
    pub async fn generate_version_id(&self) -> String {
        self.deref().generate_version_id().await
    }
    
    pub fn create_version_storage_key(&self, key: &str, version_id: &str) -> String {
        self.deref().create_version_storage_key(key, version_id)
    }
    
    pub async fn init_versioning(
        &self,
        key: &str,
        version: VersionInfo,
    ) -> Result<(), VersioningError> {
        self.deref().init_versioning(key, version).await
    }
    
    pub async fn create_version(
        &self,
        key: &str,
        version_id: &str,
        version: VersionInfo,
    ) -> Result<(), VersioningError> {
        self.deref().create_version(key, version_id, version).await
    }
    
    pub async fn get_version(
        &self,
        key: &str,
        version_id: &str,
    ) -> Result<VersionInfo, VersioningError> {
        self.deref().get_version(key, version_id).await
    }
    
    pub async fn get_version_history(&self, key: &str) -> Result<VersionHistory, VersioningError> {
        self.deref().get_version_history(key).await
    }
    
    pub async fn set_current_version(&self, key: &str, version_id: &str) -> Result<(), VersioningError> {
        self.deref().set_current_version(key, version_id).await
    }
}

impl StorageEncryptionService {
    pub fn new() -> Self {
        Self {}
    }
    
    pub async fn generate_key(&self, federations: Vec<String>) -> Result<String, EncryptionError> {
        // Generate a new encryption key and return its ID
        Ok(format!("key-{}", uuid::Uuid::new_v4()))
    }
    
    pub async fn grant_federation_key_access(&self, federation_id: &str, key_id: &str) -> Result<(), EncryptionError> {
        // Grant access to a key for a federation
        Ok(())
    }
    
    pub async fn federation_has_key_access(&self, federation_id: &str, key_id: &str) -> Result<bool, EncryptionError> {
        // Check if a federation has access to a key
        Ok(true)
    }
    
    pub async fn encrypt(&self, data: &[u8], key_id: &str) -> Result<(Vec<u8>, HashMap<String, String>), EncryptionError> {
        // Encrypt data with the specified key
        let mut metadata = HashMap::new();
        metadata.insert("key_id".to_string(), key_id.to_string());
        metadata.insert("algorithm".to_string(), "AES-256-GCM".to_string());
        Ok((data.to_vec(), metadata))
    }
    
    pub async fn decrypt(&self, data: &[u8], metadata: &HashMap<String, String>) -> Result<Vec<u8>, EncryptionError> {
        // Decrypt data with metadata
        if let Some(key_id) = metadata.get("key_id") {
            // In a real implementation, we would decrypt the data
            Ok(data.to_vec())
        } else {
            Err(EncryptionError::KeyNotFound("No key ID in metadata".to_string()))
        }
    }
}

// Implement the same methods for Arc<StorageEncryptionService>
impl<T: std::ops::Deref<Target = StorageEncryptionService> + Send + Sync> std::sync::Arc<T> {
    pub async fn generate_key(&self, federations: Vec<String>) -> Result<String, EncryptionError> {
        self.deref().generate_key(federations).await
    }
    
    pub async fn grant_federation_key_access(&self, federation_id: &str, key_id: &str) -> Result<(), EncryptionError> {
        self.deref().grant_federation_key_access(federation_id, key_id).await
    }
    
    pub async fn federation_has_key_access(&self, federation_id: &str, key_id: &str) -> Result<bool, EncryptionError> {
        self.deref().federation_has_key_access(federation_id, key_id).await
    }
    
    pub async fn encrypt(&self, data: &[u8], key_id: &str) -> Result<(Vec<u8>, HashMap<String, String>), EncryptionError> {
        self.deref().encrypt(data, key_id).await
    }
    
    pub async fn decrypt(&self, data: &[u8], metadata: &HashMap<String, String>) -> Result<Vec<u8>, EncryptionError> {
        self.deref().decrypt(data, metadata).await
    }
}

#[derive(Debug, Clone)]
pub enum QuotaOperation {
    Read { size_bytes: u64 },
    Write { size_bytes: u64 },
    Delete { key_count: u64 },
    Put { size_bytes: u64 },
} 