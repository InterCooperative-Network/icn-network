mod ml_optimizer;

use std::error::Error;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::federation::coordination::{
    FederationCoordinator,
    SharedResource,
    ResourceUsageLimits,
};
use crate::identity::Identity;
use self::ml_optimizer::MLOptimizer;

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceStatus {
    Available,
    Maintenance,
    Offline,
    Reserved,
    Depleted,
}

#[derive(Debug, Error)]
pub enum ResourceSharingError {
    #[error("Resource unavailable: {0}")]
    ResourceUnavailable(String),
    #[error("Insufficient resources: {0}")]
    InsufficientResources(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Usage limit exceeded: {0}")]
    UsageLimitExceeded(String),
    #[error("Storage error: {0}")]
    StorageError(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationPriority {
    High,
    Normal,
    Low,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AllocationStatus {
    Pending,
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAllocation {
    pub id: String,
    pub resource_id: String,
    pub federation_id: String,
    pub amount: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub status: AllocationStatus,
    pub metadata: serde_json::Value,
    pub priority: AllocationPriority,
    pub constraints: Option<ResourceConstraints>,
    pub usage_limits: Option<ResourceUsageLimits>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConstraints {
    pub max_allocation: Option<u64>,
    pub min_allocation: Option<u64>,
    pub max_duration: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsageLimits {
    pub max_concurrent_allocations: Option<u64>,
    pub max_total_allocations: Option<u64>,
    pub max_allocation_amount: Option<u64>,
    pub max_allocation_duration: Option<u64>,
}

pub struct ResourceSharingSystem {
    // ... existing fields ...
    federation_coordinator: Arc<FederationCoordinator>,
    ml_optimizer: MLOptimizer,
}

impl ResourceSharingSystem {
    pub fn new(federation_coordinator: Arc<FederationCoordinator>) -> Self {
        ResourceSharingSystem {
            // ... existing initialization ...
            federation_coordinator,
            ml_optimizer: MLOptimizer::new(),
        }
    }

    // Modify request_allocation to use ML optimizer
    pub async fn request_allocation(
        &self,
        resource_id: &str,
        amount: u64,
        duration: u64,
        metadata: serde_json::Value,
    ) -> Result<ResourceAllocation, Box<dyn Error>> {
        // Get the resource
        let mut resource: Resource = self.storage.get_json(
            &format!("resources/{}", resource_id),
        )?;

        // Check if resource is available
        if resource.status != ResourceStatus::Available {
            return Err(Box::new(ResourceSharingError::ResourceUnavailable(
                "Resource is not available".to_string(),
            )));
        }

        // Use ML optimizer to determine optimal allocation
        let (optimal_amount, optimal_duration) = self.ml_optimizer.optimize_allocation(
            resource_id,
            amount,
            duration,
            AllocationPriority::Normal, // Default to normal priority
        ).await?;

        // Check if there's enough capacity for optimal amount
        if optimal_amount > resource.capacity.total - resource.capacity.allocated - resource.capacity.reserved {
            return Err(Box::new(ResourceSharingError::InsufficientResources(
                "Not enough resource capacity available".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create allocation with optimized values
        let allocation = ResourceAllocation {
            id: format!("allocation-{}", now),
            resource_id: resource_id.to_string(),
            federation_id: self.identity.coop_id.clone(),
            amount: optimal_amount,
            start_time: now,
            end_time: now + optimal_duration,
            status: AllocationStatus::Pending,
            metadata,
            priority: AllocationPriority::Normal,
            constraints: None,
            usage_limits: None,
        };

        // Store the allocation request
        self.storage.put_json(
            &format!("allocations/{}", allocation.id),
            &allocation,
        )?;

        // Update usage patterns with new allocation
        self.update_usage_patterns(resource_id).await?;

        Ok(allocation)
    }

    // Add new method to update usage patterns
    async fn update_usage_patterns(&self, resource_id: &str) -> Result<(), Box<dyn Error>> {
        // Get historical usage data
        let allocation_ids = self.storage.list("allocations/")?;
        let mut usage_data = Vec::new();

        for id in allocation_ids {
            let allocation: ResourceAllocation = self.storage.get_json(&id)?;
            if allocation.resource_id == resource_id {
                // Convert allocation to usage data point
                usage_data.push((
                    allocation.start_time,
                    allocation.amount as f64 / self.get_resource_capacity(resource_id)? as f64,
                ));
            }
        }

        // Update ML optimizer with new usage data
        self.ml_optimizer.update_usage_pattern(resource_id, usage_data).await?;

        Ok(())
    }

    // Helper method to get resource capacity
    fn get_resource_capacity(&self, resource_id: &str) -> Result<u64, Box<dyn Error>> {
        let resource: Resource = self.storage.get_json(
            &format!("resources/{}", resource_id),
        )?;
        Ok(resource.capacity.total)
    }

    // Add cross-federation resource request handling
    pub async fn request_federation_resource(
        &self,
        resource_id: &str,
        amount: u64,
        duration: u64,
        requesting_federation: &str,
        metadata: serde_json::Value,
    ) -> Result<ResourceAllocation, Box<dyn Error>> {
        // Verify federation has access to the resource
        if !self.federation_coordinator.verify_resource_access(
            requesting_federation,
            resource_id,
        ).await? {
            return Err(Box::new(ResourceSharingError::Unauthorized(
                "Federation does not have access to this resource".to_string(),
            )));
        }

        // Get resource sharing limits
        let shared_resources = self.federation_coordinator.get_shared_resources(
            requesting_federation,
        ).await?;

        let resource_limits = shared_resources.iter()
            .find(|r| r.resource_id == resource_id)
            .ok_or("Resource sharing configuration not found")?;

        // Verify against usage limits
        self.verify_federation_usage_limits(
            resource_id,
            requesting_federation,
            amount,
            duration,
            &resource_limits.usage_limits,
        ).await?;

        // Calculate allowed amount based on share percentage
        let max_amount = (self.get_resource_capacity(resource_id)? as f64 
            * resource_limits.share_percentage) as u64;
        let requested_amount = amount.min(max_amount);

        // Use ML optimizer with federation context
        let (optimal_amount, optimal_duration) = self.ml_optimizer.optimize_allocation(
            resource_id,
            requested_amount,
            duration,
            if resource_limits.priority_access {
                AllocationPriority::High
            } else {
                AllocationPriority::Normal
            },
        ).await?;

        // Create allocation
        let allocation = ResourceAllocation {
            id: format!("fed-alloc-{}", SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()),
            resource_id: resource_id.to_string(),
            federation_id: requesting_federation.to_string(),
            amount: optimal_amount,
            start_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            end_time: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() + optimal_duration,
            status: AllocationStatus::Pending,
            metadata,
            priority: if resource_limits.priority_access {
                AllocationPriority::High
            } else {
                AllocationPriority::Normal
            },
            constraints: None,
            usage_limits: Some(resource_limits.usage_limits.clone()),
        };

        // Store allocation
        self.storage.put_json(
            &format!("allocations/{}", allocation.id),
            &allocation,
        )?;

        // Update usage patterns
        self.update_usage_patterns(resource_id).await?;

        // Update federation trust score based on resource usage
        self.update_federation_trust(requesting_federation, resource_id).await?;

        Ok(allocation)
    }

    async fn verify_federation_usage_limits(
        &self,
        resource_id: &str,
        federation_id: &str,
        requested_amount: u64,
        requested_duration: u64,
        limits: &ResourceUsageLimits,
    ) -> Result<(), Box<dyn Error>> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        let day_start = now - (now % 86400);

        // Get current allocations
        let allocations = self.get_federation_allocations(federation_id, resource_id).await?;
        
        // Check concurrent allocations
        let active_allocations = allocations.iter()
            .filter(|a| a.status == AllocationStatus::Active)
            .count() as u32;

        if active_allocations >= limits.max_concurrent_allocations {
            return Err(Box::new(ResourceSharingError::UsageLimitExceeded(
                "Maximum concurrent allocations reached".to_string(),
            )));
        }

        // Check duration limit
        if requested_duration > limits.max_duration_per_allocation {
            return Err(Box::new(ResourceSharingError::UsageLimitExceeded(
                "Requested duration exceeds maximum allowed".to_string(),
            )));
        }

        // Check daily usage
        let daily_usage = allocations.iter()
            .filter(|a| a.start_time >= day_start)
            .map(|a| a.end_time - a.start_time)
            .sum::<u64>();

        if daily_usage + requested_duration > limits.max_total_duration_per_day {
            return Err(Box::new(ResourceSharingError::UsageLimitExceeded(
                "Daily usage limit would be exceeded".to_string(),
            )));
        }

        // Check restricted hours
        let hour = ((now % 86400) / 3600) as u32;
        if limits.restricted_hours.contains(&hour) {
            return Err(Box::new(ResourceSharingError::UsageLimitExceeded(
                "Resource is not available during restricted hours".to_string(),
            )));
        }

        Ok(())
    }

    async fn get_federation_allocations(
        &self,
        federation_id: &str,
        resource_id: &str,
    ) -> Result<Vec<ResourceAllocation>, Box<dyn Error>> {
        let allocation_ids = self.storage.list("allocations/")?;
        let mut allocations = Vec::new();

        for id in allocation_ids {
            let allocation: ResourceAllocation = self.storage.get_json(&id)?;
            if allocation.federation_id == federation_id && 
               allocation.resource_id == resource_id {
                allocations.push(allocation);
            }
        }

        Ok(allocations)
    }

    async fn update_federation_trust(
        &self,
        federation_id: &str,
        resource_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Calculate trust score based on resource usage patterns
        let allocations = self.get_federation_allocations(federation_id, resource_id).await?;
        
        let mut compliance_score = 1.0;
        let mut usage_efficiency = 1.0;

        for allocation in &allocations {
            // Check if allocations were used efficiently
            if allocation.status == AllocationStatus::Completed {
                let actual_duration = allocation.end_time - allocation.start_time;
                let requested_duration = allocation.end_time - allocation.start_time;
                usage_efficiency *= (actual_duration as f64 / requested_duration as f64).min(1.0);
            }

            // Check if usage limits were respected
            if let Some(limits) = &allocation.usage_limits {
                let within_limits = actual_duration <= limits.max_duration_per_allocation;
                compliance_score *= if within_limits { 1.0 } else { 0.8 };
            }
        }

        // Update federation trust score
        self.federation_coordinator.update_trust_score(
            federation_id,
            (compliance_score + usage_efficiency) / 2.0,
        ).await?;

        Ok(())
    }

    // ... rest of existing code ...
} 