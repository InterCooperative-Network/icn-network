use std::sync::Arc;
use tempfile::tempdir;
use crate::resource_sharing::*;
use crate::identity::Identity;
use crate::storage::Storage;
use crate::crypto::CryptoUtils;

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