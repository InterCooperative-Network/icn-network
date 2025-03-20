pub mod allocation;
pub mod monitoring;
pub mod scheduling;
pub mod types;
pub mod sharing;
pub mod ml_optimizer;

// Re-export common types
pub use types::{ResourceType, ResourceConfig, Resource, ResourceQuota};
pub use allocation::{ResourceStatus, ResourceSharingError, AllocationPriority, AllocationStatus, ResourceAllocation, ResourceConstraints, ResourceUsageLimits, ResourceCapacity};
pub use monitoring::{MonitoringError, ResourceMetrics, ResourceUtilization, ResourceMonitor, ResourceThreshold, ResourceTimeSeriesData, ThresholdAction};
pub use scheduling::{SchedulingError, SchedulingRequest, ScheduledAllocation, ResourceScheduler};
pub use sharing::{SharingPolicy, SharingRule, CrossFederationRequest, CrossFederationResponse, ResourceSharingManager};
pub use ml_optimizer::{MLOptimizer, UsagePatternData};

/// Resource manager that coordinates allocation, monitoring and scheduling
pub struct ResourceManager {
    /// Resource scheduler for handling resource allocations
    scheduler: ResourceScheduler,
    /// Resource monitor for tracking resource utilization
    monitor: ResourceMonitor,
}

impl ResourceManager {
    /// Create a new resource manager
    pub fn new(scheduling_interval: u64, max_queue_size: usize) -> Self {
        Self {
            scheduler: ResourceScheduler::new(max_queue_size, scheduling_interval),
            monitor: ResourceMonitor::new(),
        }
    }
    
    /// Register a resource with the manager
    pub async fn register_resource(&self, resource: Resource) -> Result<(), Box<dyn std::error::Error>> {
        // Register with scheduler
        self.scheduler.register_resource(resource.clone()).await?;
        // Set up monitoring for the resource
        self.monitor.register_resource(resource.config.name.clone());
        Ok(())
    }
    
    /// Submit a scheduling request
    pub async fn submit_request(&self, request: SchedulingRequest) -> Result<String, Box<dyn std::error::Error>> {
        self.scheduler.submit_request(request).await
    }
    
    /// Try to immediately allocate resources for a request
    pub async fn allocate_immediately(
        &self,
        request: SchedulingRequest
    ) -> Result<ScheduledAllocation, Box<dyn std::error::Error>> {
        self.scheduler.allocate_immediately(request).await
    }
    
    /// Start the resource manager services
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Start the scheduler
        self.scheduler.start().await?;
        // Start the monitor
        self.monitor.start_monitoring().await?;
        Ok(())
    }
    
    /// Stop the resource manager services
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Stop the scheduler
        self.scheduler.stop().await?;
        // Stop the monitor
        self.monitor.stop_monitoring().await?;
        Ok(())
    }
    
    /// Get all active allocations
    pub async fn get_active_allocations(&self) -> Result<Vec<ResourceAllocation>, Box<dyn std::error::Error>> {
        self.scheduler.get_active_allocations().await
    }
    
    /// Register a threshold for a resource metric
    pub async fn register_threshold(
        &self,
        resource_id: String,
        metric: String,
        threshold: f64,
        action: ThresholdAction
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.monitor.register_threshold(resource_id, metric, threshold, action).await?;
        Ok(())
    }
    
    /// Get metrics for a specific resource
    pub async fn get_resource_metrics(
        &self,
        resource_id: &str
    ) -> Result<Option<ResourceMetrics>, Box<dyn std::error::Error>> {
        self.monitor.get_resource_metrics(resource_id).await
    }
}

// Adding a simple test for CI integration
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!(add(2, 3), 5);
    }
}
