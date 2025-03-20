use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Resource types available in the system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResourceType {
    /// Compute resources (CPU, GPU, etc.)
    Compute,
    /// Storage resources
    Storage,
    /// Network bandwidth/throughput
    Network,
    /// Memory allocation
    Memory,
    /// Custom resource type
    Custom(String),
}

/// Configuration for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// Unique name for the resource
    pub name: String,
    /// Description of the resource
    pub description: String,
    /// Type of resource
    pub resource_type: ResourceType,
    /// Total capacity of the resource
    pub capacity: f64,
    /// Additional metadata for the resource
    pub metadata: HashMap<String, String>,
}

/// Resource instance
#[derive(Debug, Clone)]
pub struct Resource {
    /// Resource configuration
    pub config: ResourceConfig,
    /// Available capacity
    pub available: f64,
    /// Currently allocated capacity
    pub allocated: f64,
}

impl Resource {
    /// Create a new resource
    pub fn new(config: ResourceConfig) -> Self {
        Self {
            available: config.capacity,
            allocated: 0.0,
            config,
        }
    }

    /// Get the total capacity
    pub fn capacity(&self) -> f64 {
        self.config.capacity
    }

    /// Get the utilization percentage (0.0-1.0)
    pub fn utilization(&self) -> f64 {
        if self.config.capacity == 0.0 {
            return 0.0;
        }
        self.allocated / self.config.capacity
    }

    /// Check if the resource has enough available capacity
    pub fn has_capacity(&self, amount: f64) -> bool {
        self.available >= amount
    }

    /// Allocate a portion of the resource
    pub fn allocate(&mut self, amount: f64) -> bool {
        if self.available >= amount {
            self.available -= amount;
            self.allocated += amount;
            true
        } else {
            false
        }
    }

    /// Release allocated capacity
    pub fn release(&mut self, amount: f64) {
        let to_release = amount.min(self.allocated);
        self.allocated -= to_release;
        self.available += to_release;
    }
}

/// Usage quota for a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceQuota {
    /// Resource identifier this quota applies to
    pub resource_id: String,
    /// Consumer identifier (user, federation, etc.)
    pub consumer_id: String,
    /// Maximum allowed allocation
    pub max_allocation: f64,
    /// Usage priority level
    pub priority: u8,
    /// Start time for the quota validity
    pub valid_from: u64,
    /// End time for the quota validity
    pub valid_until: Option<u64>,
}

impl ResourceQuota {
    /// Check if the quota is currently valid
    pub fn is_valid(&self, current_time: u64) -> bool {
        current_time >= self.valid_from && 
            self.valid_until.map_or(true, |end| current_time <= end)
    }

    /// Check if a requested allocation is within quota limits
    pub fn allows_allocation(&self, current_allocation: f64, requested_amount: f64) -> bool {
        current_allocation + requested_amount <= self.max_allocation
    }
} 