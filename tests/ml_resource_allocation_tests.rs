use std::error::Error;
use tokio::test;
use crate::resource_sharing::{
    ResourceSharingSystem,
    Resource,
    ResourceType,
    ResourceCapacity,
    ResourceStatus,
    AllocationPriority,
};

#[test]
async fn test_ml_optimizer_predictions() -> Result<(), Box<dyn Error>> {
    let system = ResourceSharingSystem::new();
    
    // Register a test resource
    let resource_id = "test-resource-1";
    system.register_resource(
        resource_id,
        "Test Resource",
        ResourceType::Computing { cores: 8, memory: 16384 },
        ResourceCapacity { total: 1000, allocated: 0, reserved: 0 },
        None,
        &"test-federation".to_string(),
        serde_json::json!({}),
    )?;

    // Create some historical allocations
    for i in 0..24 {
        let amount = if i < 8 || i > 20 { 200 } else { 600 }; // Higher usage during work hours
        system.request_allocation(
            resource_id,
            amount,
            3600, // 1 hour duration
            serde_json::json!({
                "test": "data",
                "hour": i
            }),
        ).await?;
    }

    // Request a new allocation during peak hours
    let peak_allocation = system.request_allocation(
        resource_id,
        800, // Request high amount
        3600,
        serde_json::json!({"peak": true}),
    ).await?;

    // The ML optimizer should have reduced the allocation amount during peak hours
    assert!(peak_allocation.amount < 800, 
           "ML optimizer should reduce allocation during peak hours");

    // Request allocation during off-peak hours
    let off_peak_allocation = system.request_allocation(
        resource_id,
        800,
        3600,
        serde_json::json!({"off_peak": true}),
    ).await?;

    // The ML optimizer should allow higher allocation during off-peak hours
    assert!(off_peak_allocation.amount > peak_allocation.amount,
           "ML optimizer should allow higher allocation during off-peak hours");

    Ok(())
}

#[test]
async fn test_priority_based_allocation() -> Result<(), Box<dyn Error>> {
    let system = ResourceSharingSystem::new();
    
    // Register a test resource
    let resource_id = "test-resource-2";
    system.register_resource(
        resource_id,
        "Test Resource",
        ResourceType::Computing { cores: 4, memory: 8192 },
        ResourceCapacity { total: 1000, allocated: 0, reserved: 0 },
        None,
        &"test-federation".to_string(),
        serde_json::json!({}),
    )?;

    // Create some base load
    system.request_allocation(
        resource_id,
        500, // 50% base load
        3600,
        serde_json::json!({"base_load": true}),
    ).await?;

    // Request allocations with different priorities
    let low_priority = system.request_allocation_with_priority(
        resource_id,
        300,
        3600,
        AllocationPriority::Low,
        serde_json::json!({"priority": "low"}),
    ).await?;

    let normal_priority = system.request_allocation_with_priority(
        resource_id,
        300,
        3600,
        AllocationPriority::Normal,
        serde_json::json!({"priority": "normal"}),
    ).await?;

    let high_priority = system.request_allocation_with_priority(
        resource_id,
        300,
        3600,
        AllocationPriority::High,
        serde_json::json!({"priority": "high"}),
    ).await?;

    let critical_priority = system.request_allocation_with_priority(
        resource_id,
        300,
        3600,
        AllocationPriority::Critical,
        serde_json::json!({"priority": "critical"}),
    ).await?;

    // Verify priority-based allocation behavior
    assert!(low_priority.amount < normal_priority.amount,
           "Low priority should get less resources than normal priority");
    assert!(normal_priority.amount < high_priority.amount,
           "Normal priority should get less resources than high priority");
    assert_eq!(critical_priority.amount, 300,
              "Critical priority should get exactly what was requested");

    Ok(())
}

#[test]
async fn test_adaptive_duration() -> Result<(), Box<dyn Error>> {
    let system = ResourceSharingSystem::new();
    
    // Register a test resource
    let resource_id = "test-resource-3";
    system.register_resource(
        resource_id,
        "Test Resource",
        ResourceType::Computing { cores: 2, memory: 4096 },
        ResourceCapacity { total: 1000, allocated: 0, reserved: 0 },
        None,
        &"test-federation".to_string(),
        serde_json::json!({}),
    )?;

    // Create high utilization pattern
    for _ in 0..10 {
        system.request_allocation(
            resource_id,
            800, // 80% utilization
            3600,
            serde_json::json!({"high_load": true}),
        ).await?;
    }

    // Request new allocation
    let allocation = system.request_allocation(
        resource_id,
        500,
        3600,
        serde_json::json!({"test": "adaptive_duration"}),
    ).await?;

    // The ML optimizer should have extended the duration to spread load
    assert!(allocation.end_time - allocation.start_time > 3600,
           "Duration should be extended under high load");

    // Clear existing allocations and create low utilization pattern
    system.clear_allocations(resource_id).await?;
    
    for _ in 0..10 {
        system.request_allocation(
            resource_id,
            200, // 20% utilization
            3600,
            serde_json::json!({"low_load": true}),
        ).await?;
    }

    // Request new allocation under low load
    let allocation = system.request_allocation(
        resource_id,
        500,
        3600,
        serde_json::json!({"test": "adaptive_duration_low_load"}),
    ).await?;

    // The ML optimizer should keep original duration under low load
    assert_eq!(allocation.end_time - allocation.start_time, 3600,
              "Duration should remain unchanged under low load");

    Ok(())
}

#[test]
async fn test_usage_pattern_learning() -> Result<(), Box<dyn Error>> {
    let system = ResourceSharingSystem::new();
    
    // Register a test resource
    let resource_id = "test-resource-4";
    system.register_resource(
        resource_id,
        "Test Resource",
        ResourceType::Computing { cores: 1, memory: 2048 },
        ResourceCapacity { total: 1000, allocated: 0, reserved: 0 },
        None,
        &"test-federation".to_string(),
        serde_json::json!({}),
    )?;

    // Create a weekly pattern
    for day in 0..7 {
        for hour in 0..24 {
            let amount = match (day, hour) {
                (1..=5, 9..=17) => 800, // High during work hours
                _ => 200, // Low during nights and weekends
            };

            system.request_allocation(
                resource_id,
                amount,
                3600,
                serde_json::json!({
                    "day": day,
                    "hour": hour
                }),
            ).await?;
        }
    }

    // Test allocations at different times
    let work_hour_allocation = system.request_allocation(
        resource_id,
        600,
        3600,
        serde_json::json!({"time": "work_hour"}),
    ).await?;

    let night_allocation = system.request_allocation(
        resource_id,
        600,
        3600,
        serde_json::json!({"time": "night"}),
    ).await?;

    let weekend_allocation = system.request_allocation(
        resource_id,
        600,
        3600,
        serde_json::json!({"time": "weekend"}),
    ).await?;

    // Verify that the system learned the usage patterns
    assert!(work_hour_allocation.amount < night_allocation.amount,
           "Work hour allocation should be more conservative");
    assert!(work_hour_allocation.amount < weekend_allocation.amount,
           "Work hour allocation should be more conservative than weekend");
    assert_eq!(night_allocation.amount, weekend_allocation.amount,
              "Night and weekend allocations should be similar");

    Ok(())
} 