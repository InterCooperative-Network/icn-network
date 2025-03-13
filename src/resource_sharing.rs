use std::error::Error;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};
use crate::identity::Identity;
use crate::storage::Storage;
use crate::cross_federation_governance::{CrossFederationCoordination, CoordinationType, CoordinationStatus};

// Resource sharing error types
#[derive(Debug)]
pub enum ResourceSharingError {
    InvalidResource(String),
    ResourceNotFound(String),
    InvalidFederation(String),
    InsufficientResources(String),
    ResourceUnavailable(String),
    InvalidAllocation(String),
}

impl fmt::Display for ResourceSharingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ResourceSharingError::InvalidResource(msg) => write!(f, "Invalid resource: {}", msg),
            ResourceSharingError::ResourceNotFound(msg) => write!(f, "Resource not found: {}", msg),
            ResourceSharingError::InvalidFederation(msg) => write!(f, "Invalid federation: {}", msg),
            ResourceSharingError::InsufficientResources(msg) => write!(f, "Insufficient resources: {}", msg),
            ResourceSharingError::ResourceUnavailable(msg) => write!(f, "Resource unavailable: {}", msg),
            ResourceSharingError::InvalidAllocation(msg) => write!(f, "Invalid allocation: {}", msg),
        }
    }
}

impl Error for ResourceSharingError {}

// Resource types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Computing,
    Storage,
    Network,
    Data,
    Service,
    Physical,
}

// Resource status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceStatus {
    Available,
    Allocated,
    Reserved,
    Maintenance,
    Deprecated,
}

// Resource structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,
    pub resource_type: ResourceType,
    pub name: String,
    pub description: String,
    pub owner_federation: String,
    pub capacity: ResourceCapacity,
    pub status: ResourceStatus,
    pub metadata: serde_json::Value,
    pub created_at: u64,
    pub updated_at: u64,
}

// Resource capacity structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapacity {
    pub total: u64,
    pub allocated: u64,
    pub reserved: u64,
    pub unit: String,
}

// Resource allocation structure
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
}

// Allocation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AllocationStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
    Failed,
}

// Resource sharing system
pub struct ResourceSharingSystem {
    identity: Identity,
    storage: Storage,
}

impl ResourceSharingSystem {
    // Create a new resource sharing system
    pub fn new(identity: Identity, storage: Storage) -> Self {
        ResourceSharingSystem {
            identity,
            storage,
        }
    }

    // Register a new resource
    pub fn register_resource(
        &self,
        resource_type: ResourceType,
        name: &str,
        description: &str,
        capacity: ResourceCapacity,
        metadata: serde_json::Value,
    ) -> Result<Resource, Box<dyn Error>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let resource = Resource {
            id: format!("resource-{}", now),
            resource_type,
            name: name.to_string(),
            description: description.to_string(),
            owner_federation: self.identity.coop_id.clone(),
            capacity,
            status: ResourceStatus::Available,
            metadata,
            created_at: now,
            updated_at: now,
        };

        // Store the resource
        self.storage.put_json(
            &format!("resources/{}", resource.id),
            &resource,
        )?;

        Ok(resource)
    }

    // Request resource allocation
    pub fn request_allocation(
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

        // Check if there's enough capacity
        if amount > resource.capacity.total - resource.capacity.allocated - resource.capacity.reserved {
            return Err(Box::new(ResourceSharingError::InsufficientResources(
                "Not enough resource capacity available".to_string(),
            )));
        }

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Create allocation request
        let allocation = ResourceAllocation {
            id: format!("allocation-{}", now),
            resource_id: resource_id.to_string(),
            federation_id: self.identity.coop_id.clone(),
            amount,
            start_time: now,
            end_time: now + duration,
            status: AllocationStatus::Pending,
            metadata,
        };

        // Store the allocation request
        self.storage.put_json(
            &format!("allocations/{}", allocation.id),
            &allocation,
        )?;

        Ok(allocation)
    }

    // Approve resource allocation
    pub fn approve_allocation(
        &self,
        allocation_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Get the allocation
        let mut allocation: ResourceAllocation = self.storage.get_json(
            &format!("allocations/{}", allocation_id),
        )?;

        // Get the resource
        let mut resource: Resource = self.storage.get_json(
            &format!("resources/{}", allocation.resource_id),
        )?;

        // Verify resource ownership
        if resource.owner_federation != self.identity.coop_id {
            return Err(Box::new(ResourceSharingError::InvalidFederation(
                "Not the resource owner".to_string(),
            )));
        }

        // Update resource capacity
        resource.capacity.reserved += allocation.amount;
        resource.status = ResourceStatus::Reserved;
        resource.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Update allocation status
        allocation.status = AllocationStatus::Active;

        // Store updates
        self.storage.put_json(
            &format!("resources/{}", resource.id),
            &resource,
        )?;
        self.storage.put_json(
            &format!("allocations/{}", allocation.id),
            &allocation,
        )?;

        Ok(())
    }

    // Release resource allocation
    pub fn release_allocation(
        &self,
        allocation_id: &str,
    ) -> Result<(), Box<dyn Error>> {
        // Get the allocation
        let mut allocation: ResourceAllocation = self.storage.get_json(
            &format!("allocations/{}", allocation_id),
        )?;

        // Get the resource
        let mut resource: Resource = self.storage.get_json(
            &format!("resources/{}", allocation.resource_id),
        )?;

        // Verify allocation ownership
        if allocation.federation_id != self.identity.coop_id {
            return Err(Box::new(ResourceSharingError::InvalidAllocation(
                "Not the allocation owner".to_string(),
            )));
        }

        // Update resource capacity
        resource.capacity.reserved -= allocation.amount;
        resource.status = ResourceStatus::Available;
        resource.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        // Update allocation status
        allocation.status = AllocationStatus::Completed;

        // Store updates
        self.storage.put_json(
            &format!("resources/{}", resource.id),
            &resource,
        )?;
        self.storage.put_json(
            &format!("allocations/{}", allocation.id),
            &allocation,
        )?;

        Ok(())
    }

    // Get available resources
    pub fn get_available_resources(
        &self,
        resource_type: Option<ResourceType>,
    ) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources = Vec::new();
        
        // Get all resources
        let resource_ids = self.storage.list_keys("resources/")?;
        
        for id in resource_ids {
            let resource: Resource = self.storage.get_json(&format!("resources/{}", id))?;
            
            // Filter by type if specified
            if let Some(rtype) = &resource_type {
                if resource.resource_type != *rtype {
                    continue;
                }
            }
            
            // Only include available resources
            if resource.status == ResourceStatus::Available {
                resources.push(resource);
            }
        }
        
        Ok(resources)
    }

    // Get federation's resource allocations
    pub fn get_federation_allocations(
        &self,
        status: Option<AllocationStatus>,
    ) -> Result<Vec<ResourceAllocation>, Box<dyn Error>> {
        let mut allocations = Vec::new();
        
        // Get all allocations
        let allocation_ids = self.storage.list_keys("allocations/")?;
        
        for id in allocation_ids {
            let allocation: ResourceAllocation = self.storage.get_json(&format!("allocations/{}", id))?;
            
            // Filter by federation
            if allocation.federation_id != self.identity.coop_id {
                continue;
            }
            
            // Filter by status if specified
            if let Some(astatus) = &status {
                if allocation.status != *astatus {
                    continue;
                }
            }
            
            allocations.push(allocation);
        }
        
        Ok(allocations)
    }

    // Get resource utilization metrics
    pub fn get_resource_metrics(
        &self,
        resource_id: &str,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        // Get the resource
        let resource: Resource = self.storage.get_json(
            &format!("resources/{}", resource_id),
        )?;

        // Get all allocations for this resource
        let allocation_ids = self.storage.list_keys("allocations/")?;
        let mut active_allocations = 0;
        let mut total_allocated = 0;

        for id in allocation_ids {
            let allocation: ResourceAllocation = self.storage.get_json(&format!("allocations/{}", id))?;
            if allocation.resource_id == resource_id && allocation.status == AllocationStatus::Active {
                active_allocations += 1;
                total_allocated += allocation.amount;
            }
        }

        // Create metrics
        let metrics = serde_json::json!({
            "resource_id": resource_id,
            "total_capacity": resource.capacity.total,
            "allocated_capacity": resource.capacity.allocated,
            "reserved_capacity": resource.capacity.reserved,
            "available_capacity": resource.capacity.total - resource.capacity.allocated - resource.capacity.reserved,
            "active_allocations": active_allocations,
            "total_allocated": total_allocated,
            "utilization_rate": (total_allocated as f64 / resource.capacity.total as f64) * 100.0,
            "status": resource.status,
            "last_updated": resource.updated_at,
        });

        Ok(metrics)
    }
} 