use std::error::Error;
use tokio::test;
use crate::federation::coordination::{
    FederationCoordinator,
    FederationPolicy,
    PolicyType,
    PolicyStatus,
    SharedResource,
    ResourceUsageLimits,
    AgreementStatus,
};

#[test]
async fn test_federation_registration() -> Result<(), Box<dyn Error>> {
    let coordinator = FederationCoordinator::new();

    // Create test policies
    let policies = vec![
        FederationPolicy {
            id: "policy-1".to_string(),
            policy_type: PolicyType::ResourceSharing {
                max_share_percentage: 0.5,
                priority_levels: vec!["high".to_string(), "normal".to_string()],
            },
            parameters: serde_json::json!({}),
            status: PolicyStatus::Active,
            created_at: 0,
            updated_at: 0,
        },
    ];

    // Register a federation
    let federation_id = coordinator.register_federation(
        "Test Federation",
        "A test federation",
        vec!["member1".to_string(), "member2".to_string()],
        policies,
        serde_json::json!({"test": true}),
    ).await?;

    // Verify federation was registered
    let federation_policies = coordinator.get_federation_policies(&federation_id).await?;
    assert_eq!(federation_policies.len(), 1);
    assert!(matches!(
        federation_policies[0].policy_type,
        PolicyType::ResourceSharing { .. }
    ));

    Ok(())
}

#[test]
async fn test_federation_agreement() -> Result<(), Box<dyn Error>> {
    let coordinator = FederationCoordinator::new();

    // Register two federations
    let federation_a = coordinator.register_federation(
        "Federation A",
        "First test federation",
        vec!["member1".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    let federation_b = coordinator.register_federation(
        "Federation B",
        "Second test federation",
        vec!["member2".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    // Create shared resources
    let shared_resources = vec![
        SharedResource {
            resource_id: "resource-1".to_string(),
            share_percentage: 0.3,
            priority_access: false,
            usage_limits: ResourceUsageLimits {
                max_concurrent_allocations: 5,
                max_duration_per_allocation: 3600,
                max_total_duration_per_day: 86400,
                restricted_hours: Vec::new(),
            },
        },
    ];

    // Create shared policies
    let shared_policies = vec![
        FederationPolicy {
            id: "shared-policy-1".to_string(),
            policy_type: PolicyType::ResourceSharing {
                max_share_percentage: 0.3,
                priority_levels: vec!["normal".to_string()],
            },
            parameters: serde_json::json!({}),
            status: PolicyStatus::Active,
            created_at: 0,
            updated_at: 0,
        },
    ];

    // Propose agreement
    let agreement_id = coordinator.propose_agreement(
        &federation_a,
        &federation_b,
        shared_resources,
        shared_policies,
        86400 * 30, // 30 days
    ).await?;

    // Activate agreement from both sides
    coordinator.activate_agreement(&agreement_id, &federation_a).await?;
    coordinator.activate_agreement(&agreement_id, &federation_b).await?;

    // Verify resource access
    assert!(coordinator.verify_resource_access(&federation_a, "resource-1").await?);
    assert!(coordinator.verify_resource_access(&federation_b, "resource-1").await?);

    Ok(())
}

#[test]
async fn test_trust_score_updates() -> Result<(), Box<dyn Error>> {
    let coordinator = FederationCoordinator::new();

    // Register a federation
    let federation_id = coordinator.register_federation(
        "Test Federation",
        "Trust score test federation",
        vec!["member1".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    // Update trust score multiple times
    coordinator.update_trust_score(&federation_id, 0.8).await?;
    coordinator.update_trust_score(&federation_id, 0.9).await?;
    coordinator.update_trust_score(&federation_id, 0.7).await?;

    // Get federation policies to check trust score
    let policies = coordinator.get_federation_policies(&federation_id).await?;
    
    // Trust score should be updated using exponential moving average
    // Initial score: 1.0
    // Updates: 0.8, 0.9, 0.7
    // Final score should be between 0.7 and 1.0
    let federation = coordinator.get_federation(&federation_id).await?;
    assert!(federation.trust_score > 0.7 && federation.trust_score < 1.0);

    Ok(())
}

#[test]
async fn test_agreement_suspension() -> Result<(), Box<dyn Error>> {
    let coordinator = FederationCoordinator::new();

    // Register federations
    let federation_a = coordinator.register_federation(
        "Federation A",
        "First federation",
        vec!["member1".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    let federation_b = coordinator.register_federation(
        "Federation B",
        "Second federation",
        vec!["member2".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    // Create and activate agreement
    let agreement_id = coordinator.propose_agreement(
        &federation_a,
        &federation_b,
        Vec::new(),
        Vec::new(),
        86400,
    ).await?;

    coordinator.activate_agreement(&agreement_id, &federation_a).await?;
    coordinator.activate_agreement(&agreement_id, &federation_b).await?;

    // Suspend agreement
    coordinator.suspend_agreement(
        &agreement_id,
        &federation_a,
        "Trust violation",
    ).await?;

    // Verify agreement is suspended
    let agreement = coordinator.get_agreement(&agreement_id).await?;
    assert_eq!(agreement.status, AgreementStatus::Suspended);

    // Verify suspension policy was added
    assert!(agreement.shared_policies.iter().any(|p| 
        matches!(p.policy_type, PolicyType::DisputeResolution { .. })
    ));

    // Verify resource access is denied
    assert!(!coordinator.verify_resource_access(&federation_b, "resource-1").await?);

    Ok(())
}

#[test]
async fn test_resource_sharing_limits() -> Result<(), Box<dyn Error>> {
    let coordinator = FederationCoordinator::new();

    // Register federations
    let federation_a = coordinator.register_federation(
        "Federation A",
        "Resource provider",
        vec!["member1".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    let federation_b = coordinator.register_federation(
        "Federation B",
        "Resource consumer",
        vec!["member2".to_string()],
        Vec::new(),
        serde_json::json!({}),
    ).await?;

    // Create shared resources with limits
    let shared_resources = vec![
        SharedResource {
            resource_id: "resource-1".to_string(),
            share_percentage: 0.4,
            priority_access: false,
            usage_limits: ResourceUsageLimits {
                max_concurrent_allocations: 3,
                max_duration_per_allocation: 7200,
                max_total_duration_per_day: 43200,
                restricted_hours: vec![22, 23, 0, 1, 2, 3], // Restricted during night
            },
        },
    ];

    // Create and activate agreement
    let agreement_id = coordinator.propose_agreement(
        &federation_a,
        &federation_b,
        shared_resources,
        Vec::new(),
        86400 * 7, // 1 week
    ).await?;

    coordinator.activate_agreement(&agreement_id, &federation_a).await?;
    coordinator.activate_agreement(&agreement_id, &federation_b).await?;

    // Get shared resources
    let resources = coordinator.get_shared_resources(&federation_b).await?;
    assert_eq!(resources.len(), 1);
    
    let resource = &resources[0];
    assert_eq!(resource.share_percentage, 0.4);
    assert_eq!(resource.usage_limits.max_concurrent_allocations, 3);
    assert_eq!(resource.usage_limits.restricted_hours.len(), 6);

    Ok(())
} 