use std::collections::{HashMap, VecDeque};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use serde::{Serialize, Deserialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::allocation::{ResourceAllocation, AllocationStatus, AllocationPriority};
use crate::types::{Resource, ResourceType};

/// Scheduling errors
#[derive(Debug)]
pub enum SchedulingError {
    /// No resources available for allocation
    NoResourcesAvailable(String),
    /// Resource not found
    ResourceNotFound(String),
    /// Scheduler is at capacity
    SchedulerAtCapacity(String),
    /// Invalid parameters
    InvalidParameters(String),
    /// Other error
    Other(String),
}

impl std::fmt::Display for SchedulingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulingError::NoResourcesAvailable(msg) => write!(f, "No resources available: {}", msg),
            SchedulingError::ResourceNotFound(msg) => write!(f, "Resource not found: {}", msg),
            SchedulingError::SchedulerAtCapacity(msg) => write!(f, "Scheduler at capacity: {}", msg),
            SchedulingError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
            SchedulingError::Other(msg) => write!(f, "Other scheduling error: {}", msg),
        }
    }
}

impl Error for SchedulingError {}

/// Scheduling request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingRequest {
    /// Unique request ID
    pub id: String,
    /// Federation ID making the request
    pub federation_id: String,
    /// Resource type needed
    pub resource_type: ResourceType,
    /// Amount of resource needed
    pub amount: f64,
    /// Duration in seconds
    pub duration: u64,
    /// Priority of the request
    pub priority: AllocationPriority,
    /// Earliest start time (or immediate if not specified)
    pub earliest_start_time: Option<u64>,
    /// Latest start time (or flexible if not specified)
    pub latest_start_time: Option<u64>,
    /// Additional metadata
    pub metadata: serde_json::Value,
    /// Timestamp when request was created
    pub created_at: u64,
}

impl SchedulingRequest {
    /// Create a new scheduling request
    pub fn new(
        federation_id: String,
        resource_type: ResourceType,
        amount: f64,
        duration: u64,
        priority: AllocationPriority,
        metadata: serde_json::Value,
    ) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        Self {
            id: format!("req-{}", now),
            federation_id,
            resource_type,
            amount,
            duration,
            priority,
            earliest_start_time: None,
            latest_start_time: None,
            metadata,
            created_at: now,
        }
    }
    
    /// Set time constraints for the request
    pub fn with_time_constraints(
        mut self,
        earliest_start_time: Option<u64>,
        latest_start_time: Option<u64>,
    ) -> Self {
        self.earliest_start_time = earliest_start_time;
        self.latest_start_time = latest_start_time;
        self
    }
    
    /// Check if the request can be scheduled now
    pub fn can_schedule_now(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
            
        match (self.earliest_start_time, self.latest_start_time) {
            (Some(earliest), Some(latest)) => now >= earliest && now <= latest,
            (Some(earliest), None) => now >= earliest,
            (None, Some(latest)) => now <= latest,
            (None, None) => true,
        }
    }
    
    /// Check if the request is expired (past latest start time)
    pub fn is_expired(&self) -> bool {
        if let Some(latest) = self.latest_start_time {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
                
            return now > latest;
        }
        
        false
    }
}

/// Scheduled allocation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledAllocation {
    /// The allocation that was created
    pub allocation: ResourceAllocation,
    /// The resource that was allocated
    pub resource_id: String,
    /// Start time for the allocation
    pub start_time: u64,
    /// End time for the allocation
    pub end_time: u64,
}

/// Resource scheduler for managing resource allocations
pub struct ResourceScheduler {
    /// Available resources
    resources: Arc<RwLock<HashMap<String, Resource>>>,
    /// Pending scheduling requests
    pending_requests: Arc<RwLock<VecDeque<SchedulingRequest>>>,
    /// Active allocations
    active_allocations: Arc<RwLock<HashMap<String, ResourceAllocation>>>,
    /// Is the scheduler running
    is_running: Arc<RwLock<bool>>,
    /// Maximum queue size
    max_queue_size: usize,
    /// Scheduling interval in seconds
    scheduling_interval: u64,
}

impl ResourceScheduler {
    /// Create a new resource scheduler
    pub fn new(max_queue_size: usize, scheduling_interval: u64) -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            pending_requests: Arc::new(RwLock::new(VecDeque::new())),
            active_allocations: Arc::new(RwLock::new(HashMap::new())),
            is_running: Arc::new(RwLock::new(false)),
            max_queue_size,
            scheduling_interval,
        }
    }
    
    /// Register a resource with the scheduler
    pub async fn register_resource(&self, resource: Resource) -> Result<(), Box<dyn Error>> {
        let mut resources = self.resources.write().await;
        resources.insert(resource.config.name.clone(), resource);
        Ok(())
    }
    
    /// Remove a resource from the scheduler
    pub async fn unregister_resource(&self, resource_id: &str) -> Result<(), Box<dyn Error>> {
        let mut resources = self.resources.write().await;
        resources.remove(resource_id);
        Ok(())
    }
    
    /// Submit a scheduling request
    pub async fn submit_request(&self, request: SchedulingRequest) -> Result<String, Box<dyn Error>> {
        let mut pending = self.pending_requests.write().await;
        
        // Check queue size
        if pending.len() >= self.max_queue_size {
            return Err(Box::new(SchedulingError::SchedulerAtCapacity(
                format!("Queue is at capacity ({})", self.max_queue_size)
            )));
        }
        
        // Store the request
        let request_id = request.id.clone();
        pending.push_back(request);
        
        Ok(request_id)
    }
    
    /// Try to immediately allocate resources for a request
    pub async fn allocate_immediately(
        &self,
        request: SchedulingRequest
    ) -> Result<ScheduledAllocation, Box<dyn Error>> {
        // Find a suitable resource
        let suitable_resource = self.find_suitable_resource(&request).await?;
        
        // Allocate the resource
        let allocation = self.allocate_resource(
            &request,
            &suitable_resource
        ).await?;
        
        Ok(allocation)
    }
    
    /// Start the scheduler
    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let mut is_running = self.is_running.write().await;
        if *is_running {
            return Ok(());
        }
        *is_running = true;
        drop(is_running);
        
        let pending_requests = self.pending_requests.clone();
        let resources = self.resources.clone();
        let active_allocations = self.active_allocations.clone();
        let is_running = self.is_running.clone();
        let interval_secs = self.scheduling_interval;
        
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_secs(interval_secs));
            
            loop {
                interval.tick().await;
                
                let running = *is_running.read().await;
                if !running {
                    break;
                }
                
                // Process pending requests
                if let Err(e) = Self::process_pending_requests(
                    &pending_requests,
                    &resources,
                    &active_allocations
                ).await {
                    eprintln!("Error processing pending requests: {}", e);
                }
                
                // Cleanup expired allocations
                if let Err(e) = Self::cleanup_expired_allocations(&active_allocations).await {
                    eprintln!("Error cleaning up expired allocations: {}", e);
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the scheduler
    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        let mut is_running = self.is_running.write().await;
        *is_running = false;
        Ok(())
    }
    
    /// Find a suitable resource for a request
    async fn find_suitable_resource(
        &self,
        request: &SchedulingRequest
    ) -> Result<Resource, Box<dyn Error>> {
        let resources = self.resources.read().await;
        
        // Find resources of the correct type with enough capacity
        let suitable_resources: Vec<Resource> = resources.values()
            .filter(|r| r.config.resource_type == request.resource_type && r.available >= request.amount)
            .cloned()
            .collect();
            
        if suitable_resources.is_empty() {
            return Err(Box::new(SchedulingError::NoResourcesAvailable(
                format!("No resources of type {:?} with {} available capacity", 
                       request.resource_type, request.amount)
            )));
        }
        
        // For simplicity, just pick the first one
        // In a real system, we'd have more sophisticated selection logic
        Ok(suitable_resources[0].clone())
    }
    
    /// Allocate a resource for a request
    async fn allocate_resource(
        &self,
        request: &SchedulingRequest,
        resource: &Resource
    ) -> Result<ScheduledAllocation, Box<dyn Error>> {
        // Create the allocation
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();
            
        let allocation = ResourceAllocation::new(
            resource.config.name.clone(),
            request.federation_id.clone(),
            request.amount as u64,
            request.duration,
            request.priority.clone(),
            request.metadata.clone()
        );
        
        // Update the resource
        let mut resources = self.resources.write().await;
        let resource = resources.get_mut(&resource.config.name)
            .ok_or_else(|| SchedulingError::ResourceNotFound(
                format!("Resource not found: {}", resource.config.name)
            ))?;
            
        if !resource.allocate(request.amount) {
            return Err(Box::new(SchedulingError::NoResourcesAvailable(
                format!("Resource {} no longer has enough capacity", resource.config.name)
            )));
        }
        
        // Store the allocation
        let mut active_allocations = self.active_allocations.write().await;
        active_allocations.insert(allocation.id.clone(), allocation.clone());
        
        let scheduled = ScheduledAllocation {
            allocation: allocation.clone(),
            resource_id: resource.config.name.clone(),
            start_time: now,
            end_time: now + request.duration,
        };
        
        Ok(scheduled)
    }
    
    /// Process pending scheduling requests
    async fn process_pending_requests(
        pending_requests: &Arc<RwLock<VecDeque<SchedulingRequest>>>,
        resources: &Arc<RwLock<HashMap<String, Resource>>>,
        active_allocations: &Arc<RwLock<HashMap<String, ResourceAllocation>>>
    ) -> Result<(), Box<dyn Error>> {
        let mut to_process = Vec::new();
        
        // Get all requests that can be scheduled now
        {
            let mut pending = pending_requests.write().await;
            let mut i = 0;
            
            while i < pending.len() {
                let request = &pending[i];
                
                if request.can_schedule_now() {
                    // Remove from queue and add to processing list
                    if let Some(req) = pending.remove(i) {
                        to_process.push(req);
                    }
                } else if request.is_expired() {
                    // Remove expired requests
                    pending.remove(i);
                } else {
                    // Skip to next request
                    i += 1;
                }
            }
        }
        
        // Process each request
        for request in to_process {
            // Find suitable resource
            let mut resource_found = false;
            let mut resources_map = resources.write().await;
            
            for resource in resources_map.values_mut() {
                if resource.config.resource_type == request.resource_type && resource.available >= request.amount {
                    // Found a suitable resource
                    resource_found = true;
                    
                    // Create allocation
                    let now = SystemTime::now()
                        .duration_since(UNIX_EPOCH)?
                        .as_secs();
                        
                    let allocation = ResourceAllocation::new(
                        resource.config.name.clone(),
                        request.federation_id.clone(),
                        request.amount as u64,
                        request.duration,
                        request.priority.clone(),
                        request.metadata.clone()
                    );
                    
                    // Allocate resource
                    if resource.allocate(request.amount) {
                        // Store allocation
                        let mut allocations = active_allocations.write().await;
                        allocations.insert(allocation.id.clone(), allocation);
                    }
                    
                    break;
                }
            }
            
            if !resource_found {
                // Put request back in queue if no resource found
                let mut pending = pending_requests.write().await;
                pending.push_back(request);
            }
        }
        
        Ok(())
    }
    
    /// Cleanup expired allocations
    async fn cleanup_expired_allocations(
        active_allocations: &Arc<RwLock<HashMap<String, ResourceAllocation>>>
    ) -> Result<(), Box<dyn Error>> {
        let mut expired_ids = Vec::new();
        
        // Find expired allocations
        {
            let allocations = active_allocations.read().await;
            
            for (id, allocation) in allocations.iter() {
                if allocation.is_expired() {
                    expired_ids.push(id.clone());
                }
            }
        }
        
        // Remove expired allocations
        if !expired_ids.is_empty() {
            let mut allocations = active_allocations.write().await;
            
            for id in expired_ids {
                allocations.remove(&id);
            }
        }
        
        Ok(())
    }
    
    /// Get all active allocations
    pub async fn get_active_allocations(&self) -> Result<Vec<ResourceAllocation>, Box<dyn Error>> {
        let allocations = self.active_allocations.read().await;
        Ok(allocations.values().cloned().collect())
    }
    
    /// Get active allocations for a specific federation
    pub async fn get_federation_allocations(
        &self,
        federation_id: &str
    ) -> Result<Vec<ResourceAllocation>, Box<dyn Error>> {
        let allocations = self.active_allocations.read().await;
        
        let filtered: Vec<ResourceAllocation> = allocations.values()
            .filter(|a| a.federation_id == federation_id)
            .cloned()
            .collect();
            
        Ok(filtered)
    }
    
    /// Get the status of a specific request
    pub async fn get_request_status(&self, request_id: &str) -> Result<Option<AllocationStatus>, Box<dyn Error>> {
        // Check active allocations
        let allocations = self.active_allocations.read().await;
        if let Some(allocation) = allocations.get(request_id) {
            return Ok(Some(allocation.status.clone()));
        }
        
        // Check pending requests
        let pending = self.pending_requests.read().await;
        for request in pending.iter() {
            if request.id == request_id {
                return Ok(Some(AllocationStatus::Pending));
            }
        }
        
        Ok(None)
    }
} 