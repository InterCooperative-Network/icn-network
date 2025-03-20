use std::collections::{HashMap, HashSet};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    pub max_concurrent_allocations: u32,
    pub max_duration_per_allocation: u64,
    pub max_total_duration_per_day: u64,
    pub restricted_hours: HashSet<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapacity {
    pub total: u64,
    pub allocated: u64,
    pub reserved: u64,
    pub available: u64,
}

impl ResourceCapacity {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            allocated: 0,
            reserved: 0,
            available: total,
        }
    }

    pub fn allocate(&mut self, amount: u64) -> bool {
        if amount <= self.available {
            self.allocated += amount;
            self.available -= amount;
            true
        } else {
            false
        }
    }

    pub fn release(&mut self, amount: u64) {
        let to_release = amount.min(self.allocated);
        self.allocated -= to_release;
        self.available += to_release;
    }

    pub fn reserve(&mut self, amount: u64) -> bool {
        if amount <= self.available {
            self.reserved += amount;
            self.available -= amount;
            true
        } else {
            false
        }
    }

    pub fn unreserve(&mut self, amount: u64) {
        let to_unreserve = amount.min(self.reserved);
        self.reserved -= to_unreserve;
        self.available += to_unreserve;
    }
}

impl ResourceAllocation {
    pub fn new(
        resource_id: String,
        federation_id: String,
        amount: u64,
        duration: u64,
        priority: AllocationPriority,
        metadata: serde_json::Value,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            id: format!("allocation-{}", now),
            resource_id,
            federation_id,
            amount,
            start_time: now,
            end_time: now + duration,
            status: AllocationStatus::Pending,
            metadata,
            priority,
            constraints: None,
            usage_limits: None,
        }
    }

    pub fn is_active(&self) -> bool {
        self.status == AllocationStatus::Active
    }

    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        self.end_time < now
    }

    pub fn duration(&self) -> u64 {
        self.end_time - self.start_time
    }
}

pub type ResourceSharingResult<T> = Result<T, ResourceSharingError>; 