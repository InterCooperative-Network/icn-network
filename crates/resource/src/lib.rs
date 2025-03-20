pub mod allocation;
pub mod monitoring;
pub mod scheduling;
pub mod types;

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub name: String,
    pub description: String,
    pub resource_type: ResourceType,
    pub capacity: f64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Compute,
    Storage,
    Network,
    Memory,
    Custom(String),
}

#[derive(Debug)]
pub struct ResourceManager {
    resources: Arc<RwLock<HashMap<String, Resource>>>,
    allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
}

#[derive(Debug, Clone)]
pub struct Resource {
    pub config: ResourceConfig,
    pub available: f64,
    pub allocated: f64,
}

#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    pub resource_id: String,
    pub consumer_id: String,
    pub amount: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ResourceManager {
    pub fn new() -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            allocations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_resource(&self, config: ResourceConfig) -> Result<()> {
        let resource = Resource {
            available: config.capacity,
            allocated: 0.0,
            config,
        };

        let mut resources = self.resources.write().await;
        resources.insert(resource.config.name.clone(), resource);
        Ok(())
    }

    pub async fn allocate(&self, resource_id: &str, consumer_id: &str, amount: f64) -> Result<()> {
        let mut resources = self.resources.write().await;
        let resource = resources.get_mut(resource_id).ok_or_else(|| {
            anyhow::anyhow!("Resource not found: {}", resource_id)
        })?;

        if resource.available < amount {
            return Err(anyhow::anyhow!("Insufficient resource capacity"));
        }

        resource.available -= amount;
        resource.allocated += amount;

        let allocation = ResourceAllocation {
            resource_id: resource_id.to_string(),
            consumer_id: consumer_id.to_string(),
            amount,
            timestamp: chrono::Utc::now(),
        };

        let mut allocations = self.allocations.write().await;
        allocations.insert(format!("{}:{}", resource_id, consumer_id), allocation);
        Ok(())
    }

    pub async fn deallocate(&self, resource_id: &str, consumer_id: &str) -> Result<()> {
        let allocation_key = format!("{}:{}", resource_id, consumer_id);
        let mut allocations = self.allocations.write().await;
        let allocation = allocations.remove(&allocation_key).ok_or_else(|| {
            anyhow::anyhow!("Allocation not found")
        })?;

        let mut resources = self.resources.write().await;
        let resource = resources.get_mut(resource_id).ok_or_else(|| {
            anyhow::anyhow!("Resource not found: {}", resource_id)
        })?;

        resource.available += allocation.amount;
        resource.allocated -= allocation.amount;
        Ok(())
    }
}

#[async_trait]
pub trait ResourceService {
    async fn get_resource(&self, resource_id: &str) -> Result<Resource>;
    async fn list_resources(&self) -> Result<Vec<Resource>>;
    async fn get_allocations(&self, resource_id: &str) -> Result<Vec<ResourceAllocation>>;
    async fn monitor_usage(&self, resource_id: &str) -> Result<ResourceUsage>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub resource_id: String,
    pub total_capacity: f64,
    pub used_capacity: f64,
    pub available_capacity: f64,
    pub utilization_percent: f64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
