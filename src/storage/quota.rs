use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant, SystemTime};

use crate::federation::coordination::FederationCoordinator;

/// Represents a storage quota for a federation or user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageQuota {
    /// The entity ID (federation ID or user ID)
    pub entity_id: String,
    /// The entity type (federation or user)
    pub entity_type: QuotaEntityType,
    /// Maximum storage space in bytes
    pub max_storage_bytes: u64,
    /// Maximum number of keys/objects
    pub max_keys: u64,
    /// Maximum operations per minute
    pub max_ops_per_minute: u32,
    /// Maximum bandwidth per day in bytes
    pub max_bandwidth_per_day: u64,
    /// Quota priority (higher value means higher priority)
    pub priority: u8,
    /// Whether this quota is active
    pub is_active: bool,
    /// When this quota was created
    pub created_at: u64,
    /// When this quota was last updated
    pub updated_at: u64,
    /// Custom quota properties
    pub properties: HashMap<String, String>,
}

/// The type of entity that a quota applies to
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuotaEntityType {
    /// Federation-level quota
    Federation,
    /// User-level quota
    User,
}

/// Current usage metrics for a quota
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QuotaUsage {
    /// Current storage usage in bytes
    pub storage_bytes_used: u64,
    /// Current number of keys/objects
    pub keys_used: u64,
    /// Operations in the current minute
    pub ops_this_minute: u32,
    /// Bandwidth used today in bytes
    pub bandwidth_today: u64,
    /// Start of the current minute for rate tracking
    pub minute_start: u64,
    /// Start of the current day for bandwidth tracking
    pub day_start: u64,
}

/// Represents the result of a quota check
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaCheckResult {
    /// Operation is allowed
    Allowed,
    /// Operation is throttled (temporary)
    Throttled {
        reason: String,
        retry_after_secs: u64,
    },
    /// Operation is denied (quota exceeded)
    Denied {
        reason: String,
    },
}

/// Reason for quota violation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaViolationReason {
    /// Storage space limit exceeded
    StorageLimitExceeded,
    /// Key count limit exceeded
    KeyLimitExceeded,
    /// Operation rate limit exceeded
    RateLimitExceeded,
    /// Bandwidth limit exceeded
    BandwidthLimitExceeded,
    /// Quota is inactive
    QuotaInactive,
    /// Quota not found
    QuotaNotFound,
}

impl std::fmt::Display for QuotaViolationReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StorageLimitExceeded => write!(f, "Storage limit exceeded"),
            Self::KeyLimitExceeded => write!(f, "Key limit exceeded"),
            Self::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            Self::BandwidthLimitExceeded => write!(f, "Bandwidth limit exceeded"),
            Self::QuotaInactive => write!(f, "Quota is inactive"),
            Self::QuotaNotFound => write!(f, "Quota not found"),
        }
    }
}

/// Operations that can be subject to quota checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaOperation {
    /// Store data
    Put { size_bytes: u64 },
    /// Retrieve data
    Get { size_bytes: u64 },
    /// Delete data
    Delete,
    /// List keys
    List,
    /// Get version
    GetVersion { size_bytes: u64 },
    /// Create version
    CreateVersion { size_bytes: u64 },
}

/// Storage quota system for managing resource allocation
pub struct QuotaManager {
    /// Quotas by entity ID
    quotas: RwLock<HashMap<String, StorageQuota>>,
    /// Usage tracking by entity ID
    usage: RwLock<HashMap<String, QuotaUsage>>,
    /// Priorities for scheduling
    priorities: RwLock<Vec<(String, u8)>>,
    /// Federation coordinator for resolving federation information
    federation_coordinator: Arc<FederationCoordinator>,
}

impl QuotaManager {
    /// Create a new quota manager
    pub fn new(federation_coordinator: Arc<FederationCoordinator>) -> Self {
        Self {
            quotas: RwLock::new(HashMap::new()),
            usage: RwLock::new(HashMap::new()),
            priorities: RwLock::new(Vec::new()),
            federation_coordinator,
        }
    }
    
    /// Create or update a quota
    pub async fn set_quota(&self, quota: StorageQuota) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Create modified quota with updated timestamp
        let mut updated_quota = quota.clone();
        updated_quota.updated_at = now;
        
        // Update quotas map
        {
            let mut quotas = self.quotas.write().await;
            quotas.insert(quota.entity_id.clone(), updated_quota);
        }
        
        // Update priorities list
        {
            let mut priorities = self.priorities.write().await;
            
            // Remove existing entry if any
            priorities.retain(|(id, _)| id != &quota.entity_id);
            
            // Add new priority if quota is active
            if quota.is_active {
                priorities.push((quota.entity_id.clone(), quota.priority));
                // Sort by priority (descending)
                priorities.sort_by(|(_, p1), (_, p2)| p2.cmp(p1));
            }
        }
        
        // Initialize usage tracking if it doesn't exist
        {
            let mut usage = self.usage.write().await;
            if !usage.contains_key(&quota.entity_id) {
                usage.insert(quota.entity_id.clone(), QuotaUsage::default());
            }
        }
        
        Ok(())
    }
    
    /// Get a quota by entity ID
    pub async fn get_quota(&self, entity_id: &str) -> Option<StorageQuota> {
        let quotas = self.quotas.read().await;
        quotas.get(entity_id).cloned()
    }
    
    /// Delete a quota
    pub async fn delete_quota(&self, entity_id: &str) -> Result<(), String> {
        // Remove from quotas map
        {
            let mut quotas = self.quotas.write().await;
            quotas.remove(entity_id);
        }
        
        // Remove from priorities list
        {
            let mut priorities = self.priorities.write().await;
            priorities.retain(|(id, _)| id != entity_id);
        }
        
        // Remove from usage tracking
        {
            let mut usage = self.usage.write().await;
            usage.remove(entity_id);
        }
        
        Ok(())
    }
    
    /// Get the current usage for an entity
    pub async fn get_usage(&self, entity_id: &str) -> Option<QuotaUsage> {
        let usage = self.usage.read().await;
        usage.get(entity_id).cloned()
    }
    
    /// Update usage statistics for an entity
    pub async fn update_usage(
        &self,
        entity_id: &str,
        storage_delta: i64,
        keys_delta: i64,
        ops_delta: u32,
        bandwidth_delta: u64,
    ) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        let mut usage = self.usage.write().await;
        
        // Get or create usage entry
        let entry = usage.entry(entity_id.to_string()).or_insert_with(|| {
            let mut default_usage = QuotaUsage::default();
            default_usage.minute_start = now;
            default_usage.day_start = now - (now % 86400); // Start of day (UTC)
            default_usage
        });
        
        // Check if we need to reset the minute counter
        if now - entry.minute_start >= 60 {
            entry.minute_start = now;
            entry.ops_this_minute = 0;
        }
        
        // Check if we need to reset the day counter
        let day_start = now - (now % 86400); // Start of day (UTC)
        if day_start != entry.day_start {
            entry.day_start = day_start;
            entry.bandwidth_today = 0;
        }
        
        // Update storage count (ensure it doesn't go below zero)
        if storage_delta < 0 && entry.storage_bytes_used < storage_delta.unsigned_abs() {
            entry.storage_bytes_used = 0;
        } else if storage_delta < 0 {
            entry.storage_bytes_used -= storage_delta.unsigned_abs();
        } else {
            entry.storage_bytes_used += storage_delta as u64;
        }
        
        // Update key count (ensure it doesn't go below zero)
        if keys_delta < 0 && entry.keys_used < keys_delta.unsigned_abs() {
            entry.keys_used = 0;
        } else if keys_delta < 0 {
            entry.keys_used -= keys_delta.unsigned_abs();
        } else {
            entry.keys_used += keys_delta as u64;
        }
        
        // Update operations count
        entry.ops_this_minute += ops_delta;
        
        // Update bandwidth usage
        entry.bandwidth_today += bandwidth_delta;
        
        Ok(())
    }
    
    /// Check if an operation is allowed under the quota
    pub async fn check_quota(
        &self,
        entity_id: &str,
        operation: QuotaOperation,
    ) -> QuotaCheckResult {
        // Get the quota
        let quota = {
            let quotas = self.quotas.read().await;
            match quotas.get(entity_id).cloned() {
                Some(q) => q,
                None => return QuotaCheckResult::Denied {
                    reason: QuotaViolationReason::QuotaNotFound.to_string(),
                },
            }
        };
        
        // Check if quota is active
        if !quota.is_active {
            return QuotaCheckResult::Denied {
                reason: QuotaViolationReason::QuotaInactive.to_string(),
            };
        }
        
        // Get current usage
        let usage = {
            let usage_map = self.usage.read().await;
            usage_map.get(entity_id).cloned().unwrap_or_default()
        };
        
        // Check rate limiting
        if usage.ops_this_minute >= quota.max_ops_per_minute {
            // Calculate seconds until reset
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let seconds_until_reset = if now < usage.minute_start {
                // This shouldn't happen, but just in case
                0
            } else {
                let elapsed = now - usage.minute_start;
                if elapsed >= 60 {
                    0 // Should reset now
                } else {
                    60 - elapsed
                }
            };
            
            return QuotaCheckResult::Throttled {
                reason: QuotaViolationReason::RateLimitExceeded.to_string(),
                retry_after_secs: seconds_until_reset,
            };
        }
        
        // Check bandwidth limit
        let bandwidth_required = match operation {
            QuotaOperation::Put { size_bytes } => size_bytes,
            QuotaOperation::Get { size_bytes } => size_bytes,
            QuotaOperation::GetVersion { size_bytes } => size_bytes,
            QuotaOperation::CreateVersion { size_bytes } => size_bytes,
            _ => 0,
        };
        
        if usage.bandwidth_today + bandwidth_required > quota.max_bandwidth_per_day {
            // Calculate seconds until reset
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            
            let day_start = now - (now % 86400); // Start of day (UTC)
            let seconds_until_reset = if day_start != usage.day_start {
                0 // Should reset now
            } else {
                let midnight = day_start + 86400;
                if now >= midnight {
                    0 // Should reset now
                } else {
                    midnight - now
                }
            };
            
            return QuotaCheckResult::Throttled {
                reason: QuotaViolationReason::BandwidthLimitExceeded.to_string(),
                retry_after_secs: seconds_until_reset,
            };
        }
        
        // Check storage limit for put operations
        if let QuotaOperation::Put { size_bytes } = operation {
            if usage.storage_bytes_used + size_bytes > quota.max_storage_bytes {
                return QuotaCheckResult::Denied {
                    reason: QuotaViolationReason::StorageLimitExceeded.to_string(),
                };
            }
        }
        
        // Check key limit for put operations (assuming new key)
        if let QuotaOperation::Put { .. } = operation {
            if usage.keys_used + 1 > quota.max_keys {
                return QuotaCheckResult::Denied {
                    reason: QuotaViolationReason::KeyLimitExceeded.to_string(),
                };
            }
        }
        
        // Check version creation
        if let QuotaOperation::CreateVersion { size_bytes } = operation {
            if usage.storage_bytes_used + size_bytes > quota.max_storage_bytes {
                return QuotaCheckResult::Denied {
                    reason: QuotaViolationReason::StorageLimitExceeded.to_string(),
                };
            }
        }
        
        // All checks passed
        QuotaCheckResult::Allowed
    }
    
    /// Get all quotas
    pub async fn list_quotas(&self) -> Vec<StorageQuota> {
        let quotas = self.quotas.read().await;
        quotas.values().cloned().collect()
    }
    
    /// Get all quotas for a specific entity type
    pub async fn list_quotas_by_type(&self, entity_type: QuotaEntityType) -> Vec<StorageQuota> {
        let quotas = self.quotas.read().await;
        quotas
            .values()
            .filter(|q| q.entity_type == entity_type)
            .cloned()
            .collect()
    }
    
    /// Get priorities ordered list (highest priority first)
    pub async fn get_priorities(&self) -> Vec<(String, u8)> {
        let priorities = self.priorities.read().await;
        priorities.clone()
    }
    
    /// Calculate quota utilization percentages
    pub async fn get_quota_utilization(&self, entity_id: &str) -> Option<QuotaUtilization> {
        let quota = match self.get_quota(entity_id).await {
            Some(q) => q,
            None => return None,
        };
        
        let usage = match self.get_usage(entity_id).await {
            Some(u) => u,
            None => return None,
        };
        
        let storage_percentage = if quota.max_storage_bytes == 0 {
            100.0 // Prevent division by zero
        } else {
            (usage.storage_bytes_used as f64 / quota.max_storage_bytes as f64) * 100.0
        };
        
        let keys_percentage = if quota.max_keys == 0 {
            100.0 // Prevent division by zero
        } else {
            (usage.keys_used as f64 / quota.max_keys as f64) * 100.0
        };
        
        let rate_percentage = if quota.max_ops_per_minute == 0 {
            100.0 // Prevent division by zero
        } else {
            (usage.ops_this_minute as f64 / quota.max_ops_per_minute as f64) * 100.0
        };
        
        let bandwidth_percentage = if quota.max_bandwidth_per_day == 0 {
            100.0 // Prevent division by zero
        } else {
            (usage.bandwidth_today as f64 / quota.max_bandwidth_per_day as f64) * 100.0
        };
        
        Some(QuotaUtilization {
            entity_id: entity_id.to_string(),
            storage_percentage,
            keys_percentage,
            rate_percentage,
            bandwidth_percentage,
        })
    }
    
    /// Create default federation quota
    pub async fn create_default_federation_quota(&self, federation_id: &str) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Default federation quota: 10GB storage, 10,000 keys, 1000 ops/min, 100GB bandwidth/day
        let quota = StorageQuota {
            entity_id: federation_id.to_string(),
            entity_type: QuotaEntityType::Federation,
            max_storage_bytes: 10 * 1024 * 1024 * 1024, // 10GB
            max_keys: 10_000,
            max_ops_per_minute: 1000,
            max_bandwidth_per_day: 100 * 1024 * 1024 * 1024, // 100GB
            priority: 10, // Default priority
            is_active: true,
            created_at: now,
            updated_at: now,
            properties: HashMap::new(),
        };
        
        self.set_quota(quota).await
    }
    
    /// Create default user quota
    pub async fn create_default_user_quota(&self, user_id: &str) -> Result<(), String> {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        // Default user quota: 1GB storage, 1,000 keys, 100 ops/min, 10GB bandwidth/day
        let quota = StorageQuota {
            entity_id: user_id.to_string(),
            entity_type: QuotaEntityType::User,
            max_storage_bytes: 1 * 1024 * 1024 * 1024, // 1GB
            max_keys: 1_000,
            max_ops_per_minute: 100,
            max_bandwidth_per_day: 10 * 1024 * 1024 * 1024, // 10GB
            priority: 5, // Default priority
            is_active: true,
            created_at: now,
            updated_at: now,
            properties: HashMap::new(),
        };
        
        self.set_quota(quota).await
    }
    
    /// Reset all usage counters (for testing or administrative purposes)
    pub async fn reset_all_usage(&self) -> Result<(), String> {
        let mut usage = self.usage.write().await;
        *usage = HashMap::new();
        Ok(())
    }
}

/// Quota utilization percentages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaUtilization {
    /// Entity ID
    pub entity_id: String,
    /// Storage usage percentage
    pub storage_percentage: f64,
    /// Key usage percentage
    pub keys_percentage: f64,
    /// Rate limit usage percentage
    pub rate_percentage: f64,
    /// Bandwidth usage percentage
    pub bandwidth_percentage: f64,
}

/// The priority-based scheduler for storage operations
pub struct OperationScheduler {
    /// The quota manager
    quota_manager: Arc<QuotaManager>,
    /// Pending operations by priority
    pending_operations: RwLock<HashMap<u8, Vec<PendingOperation>>>,
    /// Is the scheduler running
    running: RwLock<bool>,
}

/// A pending operation in the scheduler
#[derive(Debug, Clone)]
struct PendingOperation {
    /// The entity ID (federation or user)
    entity_id: String,
    /// The operation type
    operation: QuotaOperation,
    /// When the operation was submitted
    submitted_at: Instant,
    /// Operation callback
    callback: Arc<dyn Fn(bool) + Send + Sync>,
}

impl OperationScheduler {
    /// Create a new operation scheduler
    pub fn new(quota_manager: Arc<QuotaManager>) -> Self {
        Self {
            quota_manager,
            pending_operations: RwLock::new(HashMap::new()),
            running: RwLock::new(false),
        }
    }
    
    /// Start the scheduler
    pub async fn start(&self) -> Result<(), String> {
        let mut running = self.running.write().await;
        if *running {
            return Err("Scheduler is already running".to_string());
        }
        
        *running = true;
        
        // Spawn scheduler task
        let quota_manager = self.quota_manager.clone();
        let pending_operations = self.pending_operations.clone();
        let running_flag = self.running.clone();
        
        tokio::spawn(async move {
            while {
                let running = running_flag.read().await;
                *running
            } {
                // Process pending operations by priority
                Self::process_operations(&quota_manager, &pending_operations).await;
                
                // Sleep briefly to avoid tight loop
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
        });
        
        Ok(())
    }
    
    /// Stop the scheduler
    pub async fn stop(&self) -> Result<(), String> {
        let mut running = self.running.write().await;
        if !*running {
            return Err("Scheduler is not running".to_string());
        }
        
        *running = false;
        Ok(())
    }
    
    /// Process pending operations
    async fn process_operations(
        quota_manager: &QuotaManager,
        pending_operations: &RwLock<HashMap<u8, Vec<PendingOperation>>>,
    ) {
        // Get priorities in order (highest first)
        let priorities = quota_manager.get_priorities().await;
        
        // Get a mutable reference to pending operations
        let mut pending = pending_operations.write().await;
        
        // Process each priority level
        for (entity_id, priority) in priorities {
            if let Some(operations) = pending.get_mut(&priority) {
                // Process operations for this priority level
                let mut i = 0;
                while i < operations.len() {
                    let op = &operations[i];
                    
                    // Check if this operation is allowed by quota
                    let result = quota_manager.check_quota(&op.entity_id, op.operation).await;
                    
                    match result {
                        QuotaCheckResult::Allowed => {
                            // Update usage based on operation
                            let (storage_delta, keys_delta, ops_delta, bandwidth_delta) = 
                                Self::calculate_usage_delta(op.operation);
                            
                            let _ = quota_manager.update_usage(
                                &op.entity_id,
                                storage_delta,
                                keys_delta,
                                ops_delta,
                                bandwidth_delta,
                            ).await;
                            
                            // Execute operation callback
                            let callback = operations.remove(i).callback;
                            (callback)(true);
                            
                            // Don't increment i since we removed an element
                        }
                        QuotaCheckResult::Throttled { .. } => {
                            // Keep the operation in the queue for later
                            i += 1;
                        }
                        QuotaCheckResult::Denied { .. } => {
                            // Execute operation callback with failure
                            let callback = operations.remove(i).callback;
                            (callback)(false);
                            
                            // Don't increment i since we removed an element
                        }
                    }
                }
            }
        }
    }
    
    /// Calculate the usage delta for an operation
    fn calculate_usage_delta(operation: QuotaOperation) -> (i64, i64, u32, u64) {
        match operation {
            QuotaOperation::Put { size_bytes } => (
                size_bytes as i64,  // Storage delta
                1,                  // New key
                1,                  // One operation
                size_bytes,         // Bandwidth
            ),
            QuotaOperation::Get { size_bytes } => (
                0,                  // No storage change
                0,                  // No key change
                1,                  // One operation
                size_bytes,         // Bandwidth
            ),
            QuotaOperation::Delete => (
                -1024,              // Approximate negative storage delta (actual size unknown)
                -1,                 // Key removed
                1,                  // One operation
                1024,               // Minimal bandwidth
            ),
            QuotaOperation::List => (
                0,                  // No storage change
                0,                  // No key change
                1,                  // One operation
                4096,               // Approximate bandwidth
            ),
            QuotaOperation::GetVersion { size_bytes } => (
                0,                  // No storage change
                0,                  // No key change
                1,                  // One operation
                size_bytes,         // Bandwidth
            ),
            QuotaOperation::CreateVersion { size_bytes } => (
                size_bytes as i64,  // Storage delta
                0,                  // No key change (version isn't a new key)
                1,                  // One operation
                size_bytes,         // Bandwidth
            ),
        }
    }
    
    /// Schedule an operation with the given priority
    pub async fn schedule_operation<F>(
        &self,
        entity_id: &str,
        operation: QuotaOperation,
        callback: F,
    ) -> Result<(), String>
    where
        F: Fn(bool) + Send + Sync + 'static,
    {
        // Get the entity's priority
        let priority = {
            let quota = match self.quota_manager.get_quota(entity_id).await {
                Some(q) => q,
                None => {
                    // No quota means we can't schedule
                    return Err("No quota found for entity".to_string());
                }
            };
            
            quota.priority
        };
        
        // Create the pending operation
        let pending_op = PendingOperation {
            entity_id: entity_id.to_string(),
            operation,
            submitted_at: Instant::now(),
            callback: Arc::new(callback),
        };
        
        // Add to pending operations
        let mut pending = self.pending_operations.write().await;
        let ops = pending.entry(priority).or_insert_with(Vec::new);
        ops.push(pending_op);
        
        Ok(())
    }
    
    /// Get the number of pending operations by priority
    pub async fn get_pending_count(&self) -> HashMap<u8, usize> {
        let pending = self.pending_operations.read().await;
        pending
            .iter()
            .map(|(priority, ops)| (*priority, ops.len()))
            .collect()
    }
    
    /// Check if an operation can be executed immediately
    pub async fn can_execute_immediately(
        &self,
        entity_id: &str,
        operation: QuotaOperation,
    ) -> bool {
        match self.quota_manager.check_quota(entity_id, operation).await {
            QuotaCheckResult::Allowed => true,
            _ => false,
        }
    }
    
    /// Execute an operation immediately if allowed, otherwise return error
    pub async fn execute_immediately<F>(
        &self,
        entity_id: &str,
        operation: QuotaOperation,
        callback: F,
    ) -> Result<(), QuotaCheckResult>
    where
        F: Fn() + Send + Sync + 'static,
    {
        // Check quota
        let result = self.quota_manager.check_quota(entity_id, operation).await;
        
        match result {
            QuotaCheckResult::Allowed => {
                // Update usage based on operation
                let (storage_delta, keys_delta, ops_delta, bandwidth_delta) = 
                    Self::calculate_usage_delta(operation);
                
                let _ = self.quota_manager.update_usage(
                    entity_id,
                    storage_delta,
                    keys_delta,
                    ops_delta,
                    bandwidth_delta,
                ).await;
                
                // Execute callback
                callback();
                
                Ok(())
            }
            _ => Err(result),
        }
    }
}

/// Helper function to format size in human-readable form
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;
    
    if bytes < KB {
        format!("{} B", bytes)
    } else if bytes < MB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else if bytes < GB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes < TB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    }
} 