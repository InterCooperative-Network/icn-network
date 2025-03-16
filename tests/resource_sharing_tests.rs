use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tempfile::tempdir;
use icn_node::storage::Storage;
use icn_node::identity::Identity;
use icn_node::resource_sharing::*;
use crate::federation::coordination::{
    FederationCoordinator,
    FederationInfo,
    FederationPolicy,
    ResourceUsageLimits,
};
use tokio::sync::Arc as TokioArc;

fn setup_test() -> (ResourceSharingSystem, tempfile::TempDir) {
    let temp_dir = tempdir().unwrap();
    let storage = Storage::new(temp_dir.path().to_path_buf());
    let identity = Identity::new(
        "test-coop".to_string(),
        "test-node".to_string(),
        "test-did:test:test-coop:test-node".to_string(),
        storage.clone(),
    ).unwrap();
    let resource_sharing = ResourceSharingSystem::new(identity, storage);
    (resource_sharing, temp_dir)
}

#[test]
fn test_register_resource() {
    let (resource_sharing, _temp_dir) = setup_test();

    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let metadata = serde_json::json!({
        "location": "datacenter-1",
        "specs": {
            "cpu": "8 cores",
            "ram": "32GB"
        }
    });

    let resource = resource_sharing
        .register_resource(
            ResourceType::Computing,
            "Test Server",
            "High-performance computing server",
            capacity,
            metadata,
        )
        .unwrap();

    assert_eq!(resource.resource_type, ResourceType::Computing);
    assert_eq!(resource.name, "Test Server");
    assert_eq!(resource.capacity.total, 1000);
    assert_eq!(resource.status, ResourceStatus::Available);
    assert_eq!(resource.owner_federation, "test-coop");
}

#[test]
fn test_request_allocation() {
    let (resource_sharing, _temp_dir) = setup_test();

    // First register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage,
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Request allocation
    let allocation = resource_sharing
        .request_allocation(
            &resource.id,
            100,
            3600, // 1 hour duration
            serde_json::json!({
                "purpose": "data backup",
                "priority": "high"
            }),
        )
        .unwrap();

    assert_eq!(allocation.resource_id, resource.id);
    assert_eq!(allocation.amount, 100);
    assert_eq!(allocation.status, AllocationStatus::Pending);
    assert_eq!(allocation.federation_id, "test-coop");
}

#[test]
fn test_approve_allocation() {
    let (resource_sharing, _temp_dir) = setup_test();

    // First register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage,
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Request allocation
    let allocation = resource_sharing
        .request_allocation(
            &resource.id,
            100,
            3600,
            serde_json::json!({}),
        )
        .unwrap();

    // Approve allocation
    resource_sharing.approve_allocation(&allocation.id).unwrap();

    // Verify resource status and capacity
    let updated_resource: Resource = resource_sharing.storage
        .get_json(&format!("resources/{}", resource.id))
        .unwrap();
    
    assert_eq!(updated_resource.status, ResourceStatus::Reserved);
    assert_eq!(updated_resource.capacity.reserved, 100);

    // Verify allocation status
    let updated_allocation: ResourceAllocation = resource_sharing.storage
        .get_json(&format!("allocations/{}", allocation.id))
        .unwrap();
    
    assert_eq!(updated_allocation.status, AllocationStatus::Active);
}

#[test]
fn test_release_allocation() {
    let (resource_sharing, _temp_dir) = setup_test();

    // First register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage,
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Request and approve allocation
    let allocation = resource_sharing
        .request_allocation(
            &resource.id,
            100,
            3600,
            serde_json::json!({}),
        )
        .unwrap();
    resource_sharing.approve_allocation(&allocation.id).unwrap();

    // Release allocation
    resource_sharing.release_allocation(&allocation.id).unwrap();

    // Verify resource status and capacity
    let updated_resource: Resource = resource_sharing.storage
        .get_json(&format!("resources/{}", resource.id))
        .unwrap();
    
    assert_eq!(updated_resource.status, ResourceStatus::Available);
    assert_eq!(updated_resource.capacity.reserved, 0);

    // Verify allocation status
    let updated_allocation: ResourceAllocation = resource_sharing.storage
        .get_json(&format!("allocations/{}", allocation.id))
        .unwrap();
    
    assert_eq!(updated_allocation.status, AllocationStatus::Completed);
}

#[test]
fn test_get_available_resources() {
    let (resource_sharing, _temp_dir) = setup_test();

    // Register multiple resources
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    resource_sharing
        .register_resource(
            ResourceType::Storage,
            "Storage 1",
            "Storage resource 1",
            capacity.clone(),
            serde_json::json!({}),
        )
        .unwrap();

    resource_sharing
        .register_resource(
            ResourceType::Computing,
            "Compute 1",
            "Computing resource 1",
            capacity.clone(),
            serde_json::json!({}),
        )
        .unwrap();

    // Get all available resources
    let resources = resource_sharing.get_available_resources(None).unwrap();
    assert_eq!(resources.len(), 2);

    // Get only storage resources
    let storage_resources = resource_sharing
        .get_available_resources(Some(ResourceType::Storage))
        .unwrap();
    assert_eq!(storage_resources.len(), 1);
    assert_eq!(storage_resources[0].resource_type, ResourceType::Storage);
}

#[test]
fn test_get_federation_allocations() {
    let (resource_sharing, _temp_dir) = setup_test();

    // Register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage,
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Create multiple allocations
    let allocation1 = resource_sharing
        .request_allocation(
            &resource.id,
            100,
            3600,
            serde_json::json!({}),
        )
        .unwrap();
    resource_sharing.approve_allocation(&allocation1.id).unwrap();

    let allocation2 = resource_sharing
        .request_allocation(
            &resource.id,
            200,
            7200,
            serde_json::json!({}),
        )
        .unwrap();

    // Get all allocations
    let allocations = resource_sharing.get_federation_allocations(None).unwrap();
    assert_eq!(allocations.len(), 2);

    // Get only active allocations
    let active_allocations = resource_sharing
        .get_federation_allocations(Some(AllocationStatus::Active))
        .unwrap();
    assert_eq!(active_allocations.len(), 1);
    assert_eq!(active_allocations[0].status, AllocationStatus::Active);
}

#[test]
fn test_get_resource_metrics() {
    let (resource_sharing, _temp_dir) = setup_test();

    // Register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage,
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Create and approve an allocation
    let allocation = resource_sharing
        .request_allocation(
            &resource.id,
            300,
            3600,
            serde_json::json!({}),
        )
        .unwrap();
    resource_sharing.approve_allocation(&allocation.id).unwrap();

    // Get metrics
    let metrics = resource_sharing.get_resource_metrics(&resource.id).unwrap();
    
    assert_eq!(metrics["total_capacity"], 1000);
    assert_eq!(metrics["allocated_capacity"], 0);
    assert_eq!(metrics["reserved_capacity"], 300);
    assert_eq!(metrics["available_capacity"], 700);
    assert_eq!(metrics["active_allocations"], 1);
    assert_eq!(metrics["total_allocated"], 300);
    assert_eq!(metrics["status"], "Reserved");
}

#[test]
fn test_register_computing_resource() {
    let (resource_sharing, _temp_dir) = setup_test();

    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "cores".to_string(),
    };

    let resource_type = ResourceType::Computing {
        cpu_cores: 8,
        ram_gb: 32,
        gpu_type: Some("NVIDIA A100".to_string()),
        architecture: "x86_64".to_string(),
    };

    let metadata = serde_json::json!({
        "location": "datacenter-1",
        "specs": {
            "cpu": "8 cores",
            "ram": "32GB",
            "gpu": "NVIDIA A100"
        }
    });

    let resource = resource_sharing
        .register_resource_with_details(
            resource_type,
            "High-Performance Server",
            "GPU-enabled computing server",
            capacity,
            metadata,
        )
        .unwrap();

    assert_eq!(resource.name, "High-Performance Server");
    if let ResourceType::Computing { cpu_cores, ram_gb, .. } = resource.resource_type {
        assert_eq!(cpu_cores, 8);
        assert_eq!(ram_gb, 32);
    } else {
        panic!("Expected Computing resource type");
    }
}

#[test]
fn test_register_storage_resource() {
    let (resource_sharing, _temp_dir) = setup_test();

    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource_type = ResourceType::Storage {
        capacity_gb: 1000,
        storage_type: StorageType::NVMe,
        iops: Some(100000),
        latency_ms: Some(1),
    };

    let metadata = serde_json::json!({
        "location": "datacenter-1",
        "specs": {
            "type": "NVMe",
            "iops": 100000,
            "latency": "1ms"
        }
    });

    let resource = resource_sharing
        .register_resource_with_details(
            resource_type,
            "High-Performance Storage",
            "NVMe storage array",
            capacity,
            metadata,
        )
        .unwrap();

    assert_eq!(resource.name, "High-Performance Storage");
    if let ResourceType::Storage { storage_type, iops, .. } = resource.resource_type {
        assert_eq!(storage_type, StorageType::NVMe);
        assert_eq!(iops, Some(100000));
    } else {
        panic!("Expected Storage resource type");
    }
}

#[test]
fn test_request_allocation_with_constraints() {
    let (resource_sharing, _temp_dir) = setup_test();

    // First register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage {
                capacity_gb: 1000,
                storage_type: StorageType::SSD,
                iops: Some(50000),
                latency_ms: Some(5),
            },
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Create constraints
    let constraints = AllocationConstraints {
        min_amount: Some(100),
        max_amount: Some(500),
        preferred_time_slots: Some(vec![TimeSlot {
            start_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            end_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() + 3600,
            recurrence: Some(RecurrenceRule {
                frequency: RecurrenceFrequency::Daily,
                interval: 1,
                count: None,
                until: None,
            }),
        }]),
        required_capabilities: Some(vec!["high_iops".to_string()]),
        location_constraints: Some(vec!["datacenter-1".to_string()]),
    };

    // Create usage limits
    let usage_limits = UsageLimits {
        max_usage_per_hour: Some(100),
        max_usage_per_day: Some(1000),
        max_usage_per_week: Some(5000),
        max_usage_per_month: Some(20000),
        burst_limit: Some(200),
        cooldown_period: Some(300),
    };

    // Request allocation with constraints
    let allocation = resource_sharing
        .request_allocation_with_constraints(
            &resource.id,
            200,
            3600,
            AllocationPriority::High,
            Some(constraints),
            Some(usage_limits),
            serde_json::json!({
                "purpose": "data backup",
                "priority": "high"
            }),
        )
        .unwrap();

    assert_eq!(allocation.amount, 200);
    assert_eq!(allocation.priority, AllocationPriority::High);
    assert!(allocation.constraints.is_some());
    assert!(allocation.usage_limits.is_some());
}

#[test]
fn test_get_resources_by_capabilities() {
    let (resource_sharing, _temp_dir) = setup_test();

    // Register multiple resources with different capabilities
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    resource_sharing
        .register_resource(
            ResourceType::Storage {
                capacity_gb: 1000,
                storage_type: StorageType::SSD,
                iops: Some(50000),
                latency_ms: Some(5),
            },
            "Storage 1",
            "High-capacity storage",
            capacity.clone(),
            serde_json::json!({
                "capabilities": ["high_iops", "low_latency"]
            }),
        )
        .unwrap();

    resource_sharing
        .register_resource(
            ResourceType::Storage {
                capacity_gb: 1000,
                storage_type: StorageType::HDD,
                iops: Some(10000),
                latency_ms: Some(20),
            },
            "Storage 2",
            "High-capacity storage",
            capacity,
            serde_json::json!({
                "capabilities": ["high_capacity", "low_cost"]
            }),
        )
        .unwrap();

    // Get resources with specific capabilities
    let resources = resource_sharing
        .get_resources_by_capabilities(&["high_iops".to_string()])
        .unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].name, "Storage 1");
}

#[test]
fn test_get_resource_utilization_by_period() {
    let (resource_sharing, _temp_dir) = setup_test();

    // Register a resource
    let capacity = ResourceCapacity {
        total: 1000,
        allocated: 0,
        reserved: 0,
        unit: "GB".to_string(),
    };

    let resource = resource_sharing
        .register_resource(
            ResourceType::Storage {
                capacity_gb: 1000,
                storage_type: StorageType::SSD,
                iops: Some(50000),
                latency_ms: Some(5),
            },
            "Test Storage",
            "High-capacity storage",
            capacity,
            serde_json::json!({}),
        )
        .unwrap();

    // Create and approve multiple allocations
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let allocation1 = resource_sharing
        .request_allocation(
            &resource.id,
            100,
            3600,
            serde_json::json!({}),
        )
        .unwrap();
    resource_sharing.approve_allocation(&allocation1.id).unwrap();

    let allocation2 = resource_sharing
        .request_allocation(
            &resource.id,
            200,
            7200,
            serde_json::json!({}),
        )
        .unwrap();
    resource_sharing.approve_allocation(&allocation2.id).unwrap();

    // Get utilization metrics for a specific period
    let metrics = resource_sharing
        .get_resource_utilization_by_period(
            &resource.id,
            now,
            now + 3600,
        )
        .unwrap();

    assert_eq!(metrics["total_usage"], 300);
    assert_eq!(metrics["allocation_count"], 2);
    assert!(metrics["average_usage"].as_f64().unwrap() > 0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::federation::coordination::{
        FederationCoordinator,
        FederationInfo,
        FederationPolicy,
        ResourceUsageLimits,
    };
    use tokio::sync::Arc;

    #[tokio::test]
    async fn test_cross_federation_resource_sharing() -> Result<(), Box<dyn Error>> {
        let federation_coordinator = Arc::new(FederationCoordinator::new());
        let resource_system = ResourceSharingSystem::new(federation_coordinator.clone());

        // Register test federations
        let federation1 = FederationInfo {
            id: "fed1".to_string(),
            name: "Federation 1".to_string(),
            description: "Test federation 1".to_string(),
            members: vec!["node1".to_string(), "node2".to_string()],
            resources: vec![],
            policies: FederationPolicy::default(),
            trust_score: 1.0,
            last_active: SystemTime::now(),
            metadata: serde_json::json!({}),
        };

        let federation2 = FederationInfo {
            id: "fed2".to_string(),
            name: "Federation 2".to_string(),
            description: "Test federation 2".to_string(),
            members: vec!["node3".to_string(), "node4".to_string()],
            resources: vec![],
            policies: FederationPolicy::default(),
            trust_score: 1.0,
            last_active: SystemTime::now(),
            metadata: serde_json::json!({}),
        };

        federation_coordinator.register_federation(federation1).await?;
        federation_coordinator.register_federation(federation2).await?;

        // Set up resource sharing agreement
        let resource_id = "test-resource";
        let usage_limits = ResourceUsageLimits {
            max_concurrent_allocations: 2,
            max_duration_per_allocation: 3600,
            max_total_duration_per_day: 86400,
            restricted_hours: vec![],
        };

        federation_coordinator.create_resource_agreement(
            "fed1",
            "fed2",
            resource_id,
            0.3, // 30% share
            usage_limits.clone(),
            false, // no priority access
        ).await?;

        // Test resource request within limits
        let allocation = resource_system.request_federation_resource(
            resource_id,
            100,
            1800,
            "fed2",
            serde_json::json!({"purpose": "testing"}),
        ).await?;

        assert_eq!(allocation.federation_id, "fed2");
        assert_eq!(allocation.resource_id, resource_id);
        assert!(allocation.amount <= 100);
        assert_eq!(allocation.status, AllocationStatus::Pending);

        // Test exceeding concurrent allocation limit
        let result1 = resource_system.request_federation_resource(
            resource_id,
            100,
            1800,
            "fed2",
            serde_json::json!({}),
        ).await?;
        
        let result2 = resource_system.request_federation_resource(
            resource_id,
            100,
            1800,
            "fed2",
            serde_json::json!({}),
        ).await;

        assert!(result2.is_err());
        assert!(matches!(
            result2.unwrap_err().downcast_ref::<ResourceSharingError>(),
            Some(ResourceSharingError::UsageLimitExceeded(_))
        ));

        // Test duration limit
        let result = resource_system.request_federation_resource(
            resource_id,
            100,
            5000, // Exceeds max duration
            "fed2",
            serde_json::json!({}),
        ).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast_ref::<ResourceSharingError>(),
            Some(ResourceSharingError::UsageLimitExceeded(_))
        ));

        // Test unauthorized federation
        let result = resource_system.request_federation_resource(
            resource_id,
            100,
            1800,
            "fed3", // Unregistered federation
            serde_json::json!({}),
        ).await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err().downcast_ref::<ResourceSharingError>(),
            Some(ResourceSharingError::Unauthorized(_))
        ));

        // Test trust score impact
        let initial_trust = federation_coordinator.get_federation("fed2").await?.trust_score;
        
        // Complete an allocation efficiently
        let allocation = result1;
        resource_system.complete_allocation(&allocation.id).await?;
        
        let updated_trust = federation_coordinator.get_federation("fed2").await?.trust_score;
        assert!(updated_trust >= initial_trust);

        Ok(())
    }

    #[tokio::test]
    async fn test_federation_resource_optimization() -> Result<(), Box<dyn Error>> {
        let federation_coordinator = Arc::new(FederationCoordinator::new());
        let resource_system = ResourceSharingSystem::new(federation_coordinator.clone());

        // Register test federation with priority access
        let federation = FederationInfo {
            id: "fed_priority".to_string(),
            name: "Priority Federation".to_string(),
            description: "Test federation with priority access".to_string(),
            members: vec!["node1".to_string()],
            resources: vec![],
            policies: FederationPolicy::default(),
            trust_score: 1.0,
            last_active: SystemTime::now(),
            metadata: serde_json::json!({}),
        };

        federation_coordinator.register_federation(federation).await?;

        // Set up resource sharing agreement with priority access
        let resource_id = "priority-resource";
        let usage_limits = ResourceUsageLimits {
            max_concurrent_allocations: 5,
            max_duration_per_allocation: 7200,
            max_total_duration_per_day: 86400,
            restricted_hours: vec![],
        };

        federation_coordinator.create_resource_agreement(
            "owner_fed",
            "fed_priority",
            resource_id,
            0.5, // 50% share
            usage_limits,
            true, // priority access
        ).await?;

        // Test ML-optimized allocation with priority
        let allocation = resource_system.request_federation_resource(
            resource_id,
            200,
            3600,
            "fed_priority",
            serde_json::json!({"workload_type": "high_priority"}),
        ).await?;

        assert_eq!(allocation.priority, AllocationPriority::High);
        
        // Verify ML optimizer was used effectively
        let (amount, duration) = resource_system.ml_optimizer.get_last_optimization()?;
        assert!(amount <= 200);
        assert!(duration <= 3600);
        
        // Test allocation adjustment based on usage patterns
        for _ in 0..5 {
            let alloc = resource_system.request_federation_resource(
                resource_id,
                100,
                1800,
                "fed_priority",
                serde_json::json!({"workload_type": "regular"}),
            ).await?;
            
            resource_system.complete_allocation(&alloc.id).await?;
        }

        // Verify usage patterns are being learned
        let patterns = resource_system.ml_optimizer.get_usage_patterns(resource_id)?;
        assert!(!patterns.is_empty());

        Ok(())
    }
} 