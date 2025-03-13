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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    Computing {
        cpu_cores: u32,
        ram_gb: u32,
        gpu_type: Option<String>,
        architecture: String,
    },
    Storage {
        capacity_gb: u64,
        storage_type: StorageType,
        iops: Option<u32>,
        latency_ms: Option<u32>,
    },
    Network {
        bandwidth_mbps: u32,
        network_type: NetworkType,
        latency_ms: u32,
        region: String,
    },
    Data {
        data_type: DataType,
        size_gb: u64,
        format: String,
        retention_policy: RetentionPolicy,
    },
    Service {
        service_type: ServiceType,
        endpoints: Vec<String>,
        sla: ServiceLevelAgreement,
    },
    Physical {
        location: String,
        dimensions: Option<Dimensions>,
        weight_kg: Option<f64>,
        condition: PhysicalCondition,
    },
}

// Storage types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StorageType {
    SSD,
    HDD,
    NVMe,
    Object,
    Distributed,
}

// Network types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NetworkType {
    Ethernet,
    Fiber,
    Wireless,
    Satellite,
    Mesh,
}

// Data types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    Structured,
    Unstructured,
    TimeSeries,
    Graph,
    Multimedia,
}

// Retention policy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RetentionPolicy {
    Temporary(u64), // Duration in seconds
    Permanent,
    Custom(String),
}

// Service types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    API,
    Database,
    Cache,
    Queue,
    LoadBalancer,
    Monitoring,
}

// Service Level Agreement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServiceLevelAgreement {
    pub availability: f64, // Percentage
    pub response_time_ms: u32,
    pub support_level: SupportLevel,
    pub maintenance_window: Option<MaintenanceWindow>,
}

// Support level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SupportLevel {
    Basic,
    Standard,
    Premium,
    Enterprise,
}

// Maintenance window
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MaintenanceWindow {
    pub start_time: u64,
    pub duration_seconds: u64,
    pub frequency: MaintenanceFrequency,
}

// Maintenance frequency
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MaintenanceFrequency {
    Daily,
    Weekly,
    Monthly,
    Quarterly,
    Custom(String),
}

// Physical dimensions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Dimensions {
    pub length_cm: f64,
    pub width_cm: f64,
    pub height_cm: f64,
}

// Physical condition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PhysicalCondition {
    New,
    LikeNew,
    Good,
    Fair,
    Poor,
}

// Resource status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    pub priority: AllocationPriority,
    pub constraints: Option<AllocationConstraints>,
    pub usage_limits: Option<UsageLimits>,
}

// Allocation status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AllocationStatus {
    Pending,
    Active,
    Completed,
    Cancelled,
    Failed,
}

// Allocation priority
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AllocationPriority {
    Low,
    Normal,
    High,
    Critical,
}

// Allocation constraints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllocationConstraints {
    pub min_amount: Option<u64>,
    pub max_amount: Option<u64>,
    pub preferred_time_slots: Option<Vec<TimeSlot>>,
    pub required_capabilities: Option<Vec<String>>,
    pub location_constraints: Option<Vec<String>>,
}

// Time slot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSlot {
    pub start_time: u64,
    pub end_time: u64,
    pub recurrence: Option<RecurrenceRule>,
}

// Recurrence rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecurrenceRule {
    pub frequency: RecurrenceFrequency,
    pub interval: u32,
    pub count: Option<u32>,
    pub until: Option<u64>,
}

// Recurrence frequency
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecurrenceFrequency {
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

// Usage limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageLimits {
    pub max_usage_per_hour: Option<u64>,
    pub max_usage_per_day: Option<u64>,
    pub max_usage_per_week: Option<u64>,
    pub max_usage_per_month: Option<u64>,
    pub burst_limit: Option<u64>,
    pub cooldown_period: Option<u64>,
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
            priority: AllocationPriority::Normal,
            constraints: None,
            usage_limits: None,
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
                if !self.match_resource_type(&resource.resource_type, rtype) {
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

    // Helper method to match resource types
    fn match_resource_type(&self, a: &ResourceType, b: &ResourceType) -> bool {
        // Compare general resource type categories
        match (a, b) {
            (ResourceType::Computing { .. }, ResourceType::Computing { .. }) => true,
            (ResourceType::Storage { .. }, ResourceType::Storage { .. }) => true,
            (ResourceType::Network { .. }, ResourceType::Network { .. }) => true,
            (ResourceType::Data { .. }, ResourceType::Data { .. }) => true,
            (ResourceType::Service { .. }, ResourceType::Service { .. }) => true,
            (ResourceType::Physical { .. }, ResourceType::Physical { .. }) => true,
            _ => false,
        }
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

    // Enhanced resource registration with type-specific details
    pub fn register_resource_with_details(
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

        // Validate resource type-specific details
        self.validate_resource_details(&resource)?;

        // Store the resource
        self.storage.put_json(
            &format!("resources/{}", resource.id),
            &resource,
        )?;

        Ok(resource)
    }

    // Validate resource type-specific details
    fn validate_resource_details(&self, resource: &Resource) -> Result<(), Box<dyn Error>> {
        match &resource.resource_type {
            ResourceType::Computing { cpu_cores, ram_gb, .. } => {
                if *cpu_cores == 0 || *ram_gb == 0 {
                    return Err(Box::new(ResourceSharingError::InvalidResource(
                        "Invalid computing resource specifications".to_string(),
                    )));
                }
            }
            ResourceType::Storage { capacity_gb, .. } => {
                if *capacity_gb == 0 {
                    return Err(Box::new(ResourceSharingError::InvalidResource(
                        "Invalid storage capacity".to_string(),
                    )));
                }
            }
            ResourceType::Network { bandwidth_mbps, latency_ms, .. } => {
                if *bandwidth_mbps == 0 || *latency_ms == 0 {
                    return Err(Box::new(ResourceSharingError::InvalidResource(
                        "Invalid network specifications".to_string(),
                    )));
                }
            }
            _ => {}
        }
        Ok(())
    }

    // Enhanced allocation request with constraints
    pub fn request_allocation_with_constraints(
        &self,
        resource_id: &str,
        amount: u64,
        duration: u64,
        priority: AllocationPriority,
        constraints: Option<AllocationConstraints>,
        usage_limits: Option<UsageLimits>,
        metadata: serde_json::Value,
    ) -> Result<ResourceAllocation, Box<dyn Error>> {
        // Get the resource
        let resource: Resource = self.storage.get_json(
            &format!("resources/{}", resource_id),
        )?;

        // Validate against constraints if provided
        if let Some(constraints) = &constraints {
            if let Some(min_amount) = constraints.min_amount {
                if amount < min_amount {
                    return Err(Box::new(ResourceSharingError::InvalidAllocation(
                        "Amount below minimum constraint".to_string(),
                    )));
                }
            }
            if let Some(max_amount) = constraints.max_amount {
                if amount > max_amount {
                    return Err(Box::new(ResourceSharingError::InvalidAllocation(
                        "Amount above maximum constraint".to_string(),
                    )));
                }
            }
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
            priority,
            constraints,
            usage_limits,
        };

        // Store the allocation request
        self.storage.put_json(
            &format!("allocations/{}", allocation.id),
            &allocation,
        )?;

        Ok(allocation)
    }

    // Get resources by capabilities
    pub fn get_resources_by_capabilities(
        &self,
        required_capabilities: &[String],
    ) -> Result<Vec<Resource>, Box<dyn Error>> {
        let mut resources = Vec::new();
        
        // Get all resources
        let resource_ids = self.storage.list_keys("resources/")?;
        
        for id in resource_ids {
            let resource: Resource = self.storage.get_json(&format!("resources/{}", id))?;
            
            // Check if resource has required capabilities
            if let Some(capabilities) = resource.metadata.get("capabilities") {
                if let Some(cap_list) = capabilities.as_array() {
                    let has_all_capabilities = required_capabilities.iter().all(|cap| {
                        cap_list.iter().any(|c| c.as_str() == Some(cap))
                    });
                    
                    if has_all_capabilities && resource.status == ResourceStatus::Available {
                        resources.push(resource);
                    }
                }
            }
        }
        
        Ok(resources)
    }

    // Get resource utilization by time period
    pub fn get_resource_utilization_by_period(
        &self,
        resource_id: &str,
        start_time: u64,
        end_time: u64,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        // Get the resource
        let resource: Resource = self.storage.get_json(
            &format!("resources/{}", resource_id),
        )?;

        // Get all allocations for this resource
        let allocation_ids = self.storage.list_keys("allocations/")?;
        let mut period_allocations = Vec::new();
        let mut total_usage = 0;

        for id in allocation_ids {
            let allocation: ResourceAllocation = self.storage.get_json(&format!("allocations/{}", id))?;
            if allocation.resource_id == resource_id 
                && allocation.status == AllocationStatus::Active
                && allocation.start_time >= start_time
                && allocation.end_time <= end_time {
                period_allocations.push(allocation.clone());
                total_usage += allocation.amount;
            }
        }

        // Create utilization metrics
        let metrics = serde_json::json!({
            "resource_id": resource_id,
            "period_start": start_time,
            "period_end": end_time,
            "total_usage": total_usage,
            "average_usage": total_usage as f64 / (end_time - start_time) as f64,
            "allocation_count": period_allocations.len(),
            "allocations": period_allocations,
        });

        Ok(metrics)
    }
} 