use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use icn_network::{
    distributed_storage::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType},
    federation_storage_router::{FederationStorageRouter, StorageRoute},
    federation::{FederationStorageManager, FederationStorageConfig},
    storage::{Storage, StorageOptions},
    federation::coordination::{FederationCoordinator, FederationInfo, FederationPolicy, PolicyType, PolicyStatus},
    networking::overlay::dht::DistributedHashTable,
};

// Helper function to set up test environment
async fn setup_test_environment() -> Result<
    (
        Arc<FederationStorageManager>, 
        Arc<FederationStorageManager>,
        String,
        String
    ), 
    Box<dyn std::error::Error>
> {
    // Create a unique test directory
    let test_id = format!("test_{}", std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs());
    
    let base_dir = format!("data/test/{}", test_id);
    
    // Create storage directories
    std::fs::create_dir_all(format!("{}/fed1", base_dir)).unwrap();
    std::fs::create_dir_all(format!("{}/fed2", base_dir)).unwrap();
    
    // Create local storage instances
    let storage_fed1 = Arc::new(Storage::new(&format!("{}/fed1", base_dir)));
    let storage_fed2 = Arc::new(Storage::new(&format!("{}/fed2", base_dir)));
    
    // Create federation coordinator
    let federation_coordinator = Arc::new(FederationCoordinator::new());
    
    // Create DHT instances
    let mut dht1 = DistributedHashTable::new();
    dht1.initialize(&"test_node1".to_string(), &"addr1".into())?;
    
    let mut dht2 = DistributedHashTable::new();
    dht2.initialize(&"test_node2".to_string(), &"addr2".into())?;
    
    let dht1 = Arc::new(dht1);
    let dht2 = Arc::new(dht2);
    
    // Register federations
    let fed1_id = federation_coordinator.register_federation(
        "Test Federation 1",
        "Test federation for unit tests",
        vec!["test_node1".to_string()],
        vec![],
        serde_json::json!({"test": true}),
    ).await?;
    
    let fed2_id = federation_coordinator.register_federation(
        "Test Federation 2",
        "Test federation for unit tests",
        vec!["test_node2".to_string()],
        vec![],
        serde_json::json!({"test": true}),
    ).await?;
    
    // Create federation storage managers
    let config1 = FederationStorageConfig {
        federation_id: fed1_id.clone(),
        ..Default::default()
    };
    
    let config2 = FederationStorageConfig {
        federation_id: fed2_id.clone(),
        ..Default::default()
    };
    
    let storage_manager1 = Arc::new(FederationStorageManager::new(
        config1,
        storage_fed1,
        dht1.clone(),
        federation_coordinator.clone(),
        "test_node1".to_string(),
    ));
    
    let storage_manager2 = Arc::new(FederationStorageManager::new(
        config2,
        storage_fed2,
        dht2.clone(),
        federation_coordinator.clone(),
        "test_node2".to_string(),
    ));
    
    // Register local storage peers
    storage_manager1.register_local_peer(
        "test_node1".to_string(),
        "192.168.1.1:8000".to_string(),
        1024 * 1024 * 1024, // 1GB
        1024 * 1024 * 1024, // 1GB available
        HashMap::new(),
    ).await?;
    
    storage_manager2.register_local_peer(
        "test_node2".to_string(),
        "192.168.1.2:8000".to_string(), 
        2 * 1024 * 1024 * 1024, // 2GB
        2 * 1024 * 1024 * 1024, // 2GB available
        HashMap::new(),
    ).await?;
    
    // Setup federation agreement
    let agreement_id = federation_coordinator.propose_agreement(
        &fed1_id,
        &fed2_id,
        vec![],
        vec![],
        86400, // 1 day
    ).await?;
    
    federation_coordinator.activate_agreement(&agreement_id, &fed1_id).await?;
    federation_coordinator.activate_agreement(&agreement_id, &fed2_id).await?;
    
    Ok((storage_manager1, storage_manager2, fed1_id, fed2_id))
}

// Helper to clean up test directories
fn cleanup_test_environment(base_dir: &str) {
    let _ = std::fs::remove_dir_all(base_dir);
}

#[tokio::test]
async fn test_local_storage() -> Result<(), Box<dyn std::error::Error>> {
    let (storage_manager1, _, fed1_id, _) = setup_test_environment().await?;
    
    // Create a test policy with local federation access
    let mut policy = DataAccessPolicy::default();
    policy.read_federations.insert(fed1_id.clone());
    policy.write_federations.insert(fed1_id.clone());
    policy.admin_federations.insert(fed1_id.clone());
    
    // Store test data
    let test_data = b"Hello, Federation Storage!";
    let test_key = "test/local_storage_test.txt";
    
    storage_manager1.store_data(test_key, test_data, Some(policy)).await?;
    
    // Retrieve the data
    let retrieved_data = storage_manager1.retrieve_data(test_key).await?;
    
    // Verify the data matches
    assert_eq!(retrieved_data, test_data);
    
    Ok(())
}

#[tokio::test]
async fn test_cross_federation_storage() -> Result<(), Box<dyn std::error::Error>> {
    let (storage_manager1, storage_manager2, fed1_id, fed2_id) = setup_test_environment().await?;
    
    // Create a policy allowing both federations to read/write
    let mut policy = DataAccessPolicy::default();
    policy.read_federations.insert(fed1_id.clone());
    policy.read_federations.insert(fed2_id.clone());
    policy.write_federations.insert(fed1_id.clone());
    policy.admin_federations.insert(fed1_id.clone());
    
    // Configure routes for cross-federation access
    storage_manager1.configure_federation_route(
        "shared/".to_string(),
        vec![fed1_id.clone(), fed2_id.clone()],
        true,
        true,
        policy.clone(),
    ).await?;
    
    storage_manager2.configure_federation_route(
        "shared/".to_string(),
        vec![fed2_id.clone(), fed1_id.clone()],
        true,
        true,
        policy.clone(),
    ).await?;
    
    // Store data in Federation 1
    let test_data = b"Cross-federation test data";
    let test_key = "shared/cross_fed_test.txt";
    
    storage_manager1.store_data(test_key, test_data, Some(policy)).await?;
    
    // Try to retrieve the data from Federation 2
    let retrieved_data = storage_manager2.retrieve_data(test_key).await?;
    
    // Verify the data matches
    assert_eq!(retrieved_data, test_data);
    
    Ok(())
}

#[tokio::test]
async fn test_access_control() -> Result<(), Box<dyn std::error::Error>> {
    let (storage_manager1, storage_manager2, fed1_id, fed2_id) = setup_test_environment().await?;
    
    // Create a policy that only allows Federation 1 to access the data
    let mut restricted_policy = DataAccessPolicy::default();
    restricted_policy.read_federations.insert(fed1_id.clone());
    restricted_policy.write_federations.insert(fed1_id.clone());
    restricted_policy.admin_federations.insert(fed1_id.clone());
    
    // Store data with restricted access
    let test_data = b"Restricted access data";
    let test_key = "restricted/secure_data.txt";
    
    storage_manager1.store_data(test_key, test_data, Some(restricted_policy)).await?;
    
    // Federation 1 should be able to read the data
    let result1 = storage_manager1.retrieve_data(test_key).await;
    assert!(result1.is_ok());
    
    // Federation 2 should NOT be able to read the data
    let result2 = storage_manager2.retrieve_data(test_key).await;
    assert!(result2.is_err());
    
    Ok(())
}

#[tokio::test]
async fn test_storage_statistics() -> Result<(), Box<dyn std::error::Error>> {
    let (storage_manager1, _, _, _) = setup_test_environment().await?;
    
    // Get initial stats
    let initial_stats = storage_manager1.get_federation_storage_stats().await?;
    
    // Verify expected values
    assert_eq!(initial_stats.peer_count, 1);
    assert_eq!(initial_stats.total_capacity, 1024 * 1024 * 1024); // 1GB
    assert_eq!(initial_stats.available_space, 1024 * 1024 * 1024); // 1GB
    assert!(initial_stats.utilization_percentage < 0.01); // Should be close to 0%
    
    Ok(())
}

#[tokio::test]
async fn test_peer_updates() -> Result<(), Box<dyn std::error::Error>> {
    let (storage_manager1, _, _, _) = setup_test_environment().await?;
    
    // Update peer space
    let new_available_space = 512 * 1024 * 1024; // 512MB
    storage_manager1.update_peer_space("test_node1", new_available_space).await?;
    
    // Get updated stats
    let updated_stats = storage_manager1.get_federation_storage_stats().await?;
    
    // Verify updates were applied
    assert_eq!(updated_stats.available_space, new_available_space);
    assert!(updated_stats.utilization_percentage > 0.01); // Should be > 0% now
    
    Ok(())
} 