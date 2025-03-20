use std::collections::HashMap;
use std::sync::Arc;
use std::error::Error;
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::allocation::{ResourceAllocation, AllocationPriority, ResourceConstraints};
use crate::scheduling::{SchedulingRequest, ResourceScheduler};
use crate::types::{ResourceType, Resource};
use crate::ml_optimizer::MLOptimizer;

/// Federation resource sharing error
#[derive(Debug, thiserror::Error)]
pub enum ResourceSharingError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),
    
    #[error("Insufficient capacity: {0}")]
    InsufficientCapacity(String),
    
    #[error("Invalid allocation: {0}")]
    InvalidAllocation(String),
    
    #[error("Authorization error: {0}")]
    AuthorizationError(String),
    
    #[error("Federation not found: {0}")]
    FederationNotFound(String),
    
    #[error("Cross-federation request failed: {0}")]
    CrossFederationRequestFailed(String),
    
    #[error("Scheduler error: {0}")]
    SchedulerError(String),
    
    #[error("Other error: {0}")]
    Other(String),
}

/// Resource sharing policy between federations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SharingPolicy {
    /// Open sharing with few restrictions
    Open,
    /// Restricted sharing with specific rules
    Restricted(Vec<SharingRule>),
    /// Closed sharing (only within federation)
    Closed,
    /// Custom policy with specific configuration
    Custom(serde_json::Value),
}

/// Rule for resource sharing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharingRule {
    /// The federation ID this rule applies to
    pub federation_id: String,
    /// Maximum amount that can be allocated (as a percentage of total)
    pub max_allocation_percent: f64,
    /// Priority level for allocations
    pub priority: AllocationPriority,
    /// Resource types allowed to be shared
    pub allowed_resource_types: Vec<ResourceType>,
    /// Additional restrictions as key-value pairs
    pub restrictions: serde_json::Value,
}

/// Cross-federation resource request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFederationRequest {
    /// Unique request ID
    pub id: String,
    /// Source federation ID
    pub source_federation_id: String,
    /// Target federation ID
    pub target_federation_id: String,
    /// Resource type needed
    pub resource_type: ResourceType,
    /// Amount of resource needed
    pub amount: f64,
    /// Duration in seconds
    pub duration: u64,
    /// Priority of the request
    pub priority: AllocationPriority,
    /// Constraints on the allocation
    pub constraints: Option<ResourceConstraints>,
    /// Timestamp when request was created
    pub created_at: u64,
    /// Timestamp when request expires
    pub expires_at: u64,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl CrossFederationRequest {
    /// Create a new cross-federation request
    pub fn new(
        source_federation_id: String,
        target_federation_id: String,
        resource_type: ResourceType,
        amount: f64,
        duration: u64,
        priority: AllocationPriority,
        constraints: Option<ResourceConstraints>,
        metadata: serde_json::Value,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id: format!("cfr-{}", now),
            source_federation_id,
            target_federation_id,
            resource_type,
            amount,
            duration,
            priority,
            constraints,
            created_at: now,
            expires_at: now + 3600, // Default expiration of 1 hour
            metadata,
        }
    }
    
    /// Set a custom expiration time
    pub fn with_expiration(mut self, expires_at: u64) -> Self {
        self.expires_at = expires_at;
        self
    }
    
    /// Convert to a scheduling request for the target federation
    pub fn to_scheduling_request(&self) -> SchedulingRequest {
        SchedulingRequest::new(
            self.source_federation_id.clone(),
            self.resource_type.clone(),
            self.amount,
            self.duration,
            self.priority.clone(),
            self.metadata.clone(),
        )
    }
}

/// Response to a cross-federation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossFederationResponse {
    /// The request ID this is responding to
    pub request_id: String,
    /// Whether the request was accepted
    pub accepted: bool,
    /// If accepted, the allocation ID
    pub allocation_id: Option<String>,
    /// If rejected, the reason
    pub rejection_reason: Option<String>,
    /// Additional terms or conditions
    pub terms: Option<serde_json::Value>,
    /// Timestamp of the response
    pub timestamp: u64,
}

/// Resource sharing manager for handling sharing between federations
pub struct ResourceSharingManager {
    /// Resource scheduler
    scheduler: Arc<ResourceScheduler>,
    /// Sharing policies per federation
    sharing_policies: Arc<RwLock<HashMap<String, SharingPolicy>>>,
    /// Pending cross-federation requests
    pending_requests: Arc<RwLock<HashMap<String, CrossFederationRequest>>>,
    /// ML optimizer for resource requests
    ml_optimizer: MLOptimizer,
}

impl ResourceSharingManager {
    /// Create a new resource sharing manager
    pub fn new(scheduler: Arc<ResourceScheduler>) -> Self {
        Self {
            scheduler,
            sharing_policies: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
            ml_optimizer: MLOptimizer::new(),
        }
    }
    
    /// Set a sharing policy for a federation
    pub async fn set_sharing_policy(&self, federation_id: String, policy: SharingPolicy) -> Result<(), Box<dyn Error>> {
        let mut policies = self.sharing_policies.write().await;
        policies.insert(federation_id, policy);
        Ok(())
    }
    
    /// Get a federation's sharing policy
    pub async fn get_sharing_policy(&self, federation_id: &str) -> Option<SharingPolicy> {
        let policies = self.sharing_policies.read().await;
        policies.get(federation_id).cloned()
    }
    
    /// Request allocation from another federation
    pub async fn request_allocation(
        &self,
        request: CrossFederationRequest
    ) -> Result<CrossFederationResponse, Box<dyn Error>> {
        // Store the pending request
        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(request.id.clone(), request.clone());
        }
        
        // In a real implementation, this would contact the target federation
        // For now, we'll simulate the response
        
        // Check if the target federation allows sharing with the source federation
        let can_share = self.check_sharing_allowed(
            &request.target_federation_id,
            &request.source_federation_id,
            &request.resource_type,
            request.amount
        ).await;
        
        let response = if can_share {
            // Convert to a scheduling request and submit it
            let scheduling_request = request.to_scheduling_request();
            match self.scheduler.submit_request(scheduling_request).await {
                Ok(allocation_id) => CrossFederationResponse {
                    request_id: request.id,
                    accepted: true,
                    allocation_id: Some(allocation_id),
                    rejection_reason: None,
                    terms: None,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                },
                Err(e) => CrossFederationResponse {
                    request_id: request.id,
                    accepted: false,
                    allocation_id: None,
                    rejection_reason: Some(format!("Scheduling failed: {}", e)),
                    terms: None,
                    timestamp: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs(),
                }
            }
        } else {
            CrossFederationResponse {
                request_id: request.id,
                accepted: false,
                allocation_id: None,
                rejection_reason: Some("Sharing not allowed by policy".to_string()),
                terms: None,
                timestamp: SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs(),
            }
        };
        
        Ok(response)
    }
    
    /// Check if sharing is allowed based on policies
    async fn check_sharing_allowed(
        &self,
        target_federation_id: &str,
        source_federation_id: &str,
        resource_type: &ResourceType,
        amount: f64
    ) -> bool {
        let policies = self.sharing_policies.read().await;
        
        // Get target federation's policy
        if let Some(policy) = policies.get(target_federation_id) {
            match policy {
                SharingPolicy::Open => true,
                SharingPolicy::Closed => false,
                SharingPolicy::Restricted(rules) => {
                    // Check if there's a rule for the source federation
                    for rule in rules {
                        if rule.federation_id == source_federation_id {
                            // Check if the resource type is allowed
                            if rule.allowed_resource_types.contains(resource_type) {
                                return true;
                            }
                        }
                    }
                    false
                },
                SharingPolicy::Custom(_) => {
                    // For custom policies, we would have more complex logic
                    // For simplicity, default to allowing
                    true
                }
            }
        } else {
            // No policy means default to closed
            false
        }
    }
    
    /// Process a response from another federation
    pub async fn process_response(
        &self,
        response: CrossFederationResponse
    ) -> Result<(), Box<dyn Error>> {
        // Get the original request
        let mut pending = self.pending_requests.write().await;
        let request = pending.remove(&response.request_id)
            .ok_or_else(|| ResourceSharingError::Other(
                format!("Request not found: {}", response.request_id)
            ))?;
        
        // In a real implementation, we would handle the response
        // For now, we'll just log it
        if response.accepted {
            println!("Request {} accepted by federation {}", 
                     request.id, request.target_federation_id);
            if let Some(allocation_id) = &response.allocation_id {
                println!("Allocation ID: {}", allocation_id);
            }
        } else {
            println!("Request {} rejected by federation {}", 
                     request.id, request.target_federation_id);
            if let Some(reason) = &response.rejection_reason {
                println!("Reason: {}", reason);
            }
        }
        
        Ok(())
    }
    
    /// Optimize a resource allocation request
    pub async fn optimize_allocation(
        &self,
        federation_id: &str,
        resource_type: ResourceType,
        base_amount: f64,
        duration: u64,
        priority: AllocationPriority
    ) -> Result<SchedulingRequest, Box<dyn Error>> {
        // Use the ML optimizer to optimize the request
        let optimized_amount = self.ml_optimizer.optimize_allocation(
            federation_id.to_string(),
            resource_type.clone(),
            base_amount,
            priority.clone()
        );
        
        // Create a new scheduling request with the optimized amount
        let request = SchedulingRequest::new(
            federation_id.to_string(),
            resource_type,
            optimized_amount,
            duration,
            priority,
            serde_json::json!({
                "optimized": true,
                "original_amount": base_amount
            })
        );
        
        Ok(request)
    }
    
    /// Update usage pattern for optimization
    pub async fn update_usage_pattern(
        &self,
        federation_id: &str,
        resource_type: &ResourceType,
        amount: f64,
        timestamp: u64
    ) -> Result<(), Box<dyn Error>> {
        self.ml_optimizer.update_usage_pattern(
            federation_id.to_string(),
            resource_type.clone(),
            amount,
            timestamp
        );
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_sharing_policy() {
        let scheduler = Arc::new(ResourceScheduler::new(100, 10));
        let manager = ResourceSharingManager::new(scheduler);
        
        // Set an open policy for federation1
        manager.set_sharing_policy(
            "federation1".to_string(),
            SharingPolicy::Open
        ).await.unwrap();
        
        // Set a restricted policy for federation2
        let rule = SharingRule {
            federation_id: "federation1".to_string(),
            max_allocation_percent: 20.0,
            priority: AllocationPriority::Normal,
            allowed_resource_types: vec![ResourceType::Compute, ResourceType::Memory],
            restrictions: serde_json::json!({}),
        };
        
        manager.set_sharing_policy(
            "federation2".to_string(),
            SharingPolicy::Restricted(vec![rule])
        ).await.unwrap();
        
        // Check sharing allowed
        let allowed1 = manager.check_sharing_allowed(
            "federation1",
            "federation2",
            &ResourceType::Compute,
            10.0
        ).await;
        
        assert!(allowed1);
        
        // Check sharing allowed with restricted policy
        let allowed2 = manager.check_sharing_allowed(
            "federation2",
            "federation1",
            &ResourceType::Compute,
            10.0
        ).await;
        
        assert!(allowed2);
        
        // Check sharing not allowed for disallowed resource type
        let allowed3 = manager.check_sharing_allowed(
            "federation2",
            "federation1",
            &ResourceType::Storage,
            10.0
        ).await;
        
        assert!(!allowed3);
    }
} 