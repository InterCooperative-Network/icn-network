use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

use icn_storage::distributed::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType};
use icn_storage::federation::{FederationStorageRouter, StorageRoute};
use icn_storage::federation::{FederationStorageManager, FederationStorageConfig};
use icn_core::storage::{Storage, StorageOptions};
use icn_governance::federation::coordination::{FederationCoordinator, FederationInfo, FederationPolicy, PolicyType, PolicyStatus};
use icn_network::overlay::dht::DistributedHashTable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ICN Distributed Storage Demo");
    println!("============================\n");

    // Setup storage directories
    println!("Setting up storage directories...");
    let base_dir = "data/storage_demo";
    std::fs::create_dir_all(format!("{}/fed1", base_dir))?;
    std::fs::create_dir_all(format!("{}/fed2", base_dir))?;
    std::fs::create_dir_all(format!("{}/fed3", base_dir))?;
    println!("Storage directories created.");

    // Create local storage instances
    println!("Creating federation infrastructure...");
    let storage_fed1 = Arc::new(Storage::new(&format!("{}/fed1", base_dir)));
    let storage_fed2 = Arc::new(Storage::new(&format!("{}/fed2", base_dir)));
    let storage_fed3 = Arc::new(Storage::new(&format!("{}/fed3", base_dir)));

    // Create federation coordinator
    let federation_coordinator = Arc::new(FederationCoordinator::new());
    
    // Create DHT instances for lookups
    let mut dht1 = DistributedHashTable::new();
    dht1.initialize(&"node1".to_string(), &"addr1".into())?;
    
    let mut dht2 = DistributedHashTable::new();
    dht2.initialize(&"node2".to_string(), &"addr2".into())?;
    
    let mut dht3 = DistributedHashTable::new();
    dht3.initialize(&"node3".to_string(), &"addr3".into())?;
    
    let dht1 = Arc::new(dht1);
    let dht2 = Arc::new(dht2);
    let dht3 = Arc::new(dht3);

    // Register federations
    println!("Registering federations...");
    let fed1_id = federation_coordinator.register_federation(
        "Federation 1",
        "Data processing federation",
        vec!["node1".to_string()],
        vec![],
        serde_json::json!({"region": "east"}),
    ).await?;
    
    let fed2_id = federation_coordinator.register_federation(
        "Federation 2",
        "Storage federation",
        vec!["node2".to_string()],
        vec![],
        serde_json::json!({"region": "west"}),
    ).await?;
    
    let fed3_id = federation_coordinator.register_federation(
        "Federation 3",
        "Analytics federation",
        vec!["node3".to_string()],
        vec![],
        serde_json::json!({"region": "north"}),
    ).await?;
    
    println!("Registered federations: {} {} {}", fed1_id, fed2_id, fed3_id);

    // Create federation storage managers
    println!("Creating federation storage managers...");
    let config1 = FederationStorageConfig {
        federation_id: fed1_id.clone(),
        ..Default::default()
    };
    
    let config2 = FederationStorageConfig {
        federation_id: fed2_id.clone(),
        ..Default::default()
    };
    
    let config3 = FederationStorageConfig {
        federation_id: fed3_id.clone(),
        ..Default::default()
    };
    
    let storage_manager1 = FederationStorageManager::new(
        config1,
        storage_fed1,
        dht1.clone(),
        federation_coordinator.clone(),
        "node1".to_string(),
    );
    
    let storage_manager2 = FederationStorageManager::new(
        config2,
        storage_fed2,
        dht2.clone(),
        federation_coordinator.clone(),
        "node2".to_string(),
    );
    
    let storage_manager3 = FederationStorageManager::new(
        config3,
        storage_fed3,
        dht3.clone(),
        federation_coordinator.clone(), 
        "node3".to_string(),
    );

    // Register local storage peers
    println!("Registering storage peers...");
    storage_manager1.register_local_peer(
        "node1".to_string(),
        "192.168.1.1:8000".to_string(),
        1024 * 1024 * 1024, // 1GB
        1024 * 1024 * 1024, // 1GB available
        HashMap::new(),
    ).await?;
    
    storage_manager2.register_local_peer(
        "node2".to_string(),
        "192.168.1.2:8000".to_string(), 
        2 * 1024 * 1024 * 1024, // 2GB
        2 * 1024 * 1024 * 1024, // 2GB available
        HashMap::new(),
    ).await?;
    
    storage_manager3.register_local_peer(
        "node3".to_string(),
        "192.168.1.3:8000".to_string(),
        3 * 1024 * 1024 * 1024, // 3GB
        3 * 1024 * 1024 * 1024, // 3GB available
        HashMap::new(),
    ).await?;
    
    // Setup federation agreements
    println!("Setting up cross-federation agreements...");
    
    // Create an agreement between Federation 1 and Federation 2
    let agreement12_id = federation_coordinator.propose_agreement(
        &fed1_id,
        &fed2_id,
        vec![],
        vec![],
        86400 * 30, // 30 days
    ).await?;
    
    federation_coordinator.activate_agreement(&agreement12_id, &fed1_id).await?;
    federation_coordinator.activate_agreement(&agreement12_id, &fed2_id).await?;
    
    // Create an agreement between Federation 2 and Federation 3
    let agreement23_id = federation_coordinator.propose_agreement(
        &fed2_id,
        &fed3_id,
        vec![],
        vec![],
        86400 * 30, // 30 days
    ).await?;
    
    federation_coordinator.activate_agreement(&agreement23_id, &fed2_id).await?;
    federation_coordinator.activate_agreement(&agreement23_id, &fed3_id).await?;
    
    println!("Federation agreements activated.");

    // Configure federation routes
    println!("Configuring federation storage routes...");
    
    // Create access policies for each federation
    let mut fed1_policy = DataAccessPolicy::default();
    fed1_policy.read_federations.insert(fed1_id.clone());
    fed1_policy.write_federations.insert(fed1_id.clone());
    fed1_policy.admin_federations.insert(fed1_id.clone());
    
    let mut fed2_policy = DataAccessPolicy::default();
    fed2_policy.read_federations.insert(fed2_id.clone());
    fed2_policy.read_federations.insert(fed1_id.clone()); // Fed1 can read from Fed2
    fed2_policy.write_federations.insert(fed2_id.clone());
    fed2_policy.admin_federations.insert(fed2_id.clone());
    
    let mut fed3_policy = DataAccessPolicy::default();
    fed3_policy.read_federations.insert(fed3_id.clone());
    fed3_policy.read_federations.insert(fed2_id.clone()); // Fed2 can read from Fed3
    fed3_policy.write_federations.insert(fed3_id.clone());
    fed3_policy.admin_federations.insert(fed3_id.clone());
    
    // Configure storage routes
    storage_manager1.configure_federation_route(
        "data/shared/".to_string(),
        vec![fed1_id.clone(), fed2_id.clone()],
        true,
        true,
        fed1_policy.clone(),
    ).await?;
    
    storage_manager2.configure_federation_route(
        "data/shared/".to_string(),
        vec![fed2_id.clone(), fed1_id.clone()],
        true,
        true,
        fed2_policy.clone(),
    ).await?;
    
    storage_manager3.configure_federation_route(
        "data/analytics/".to_string(),
        vec![fed3_id.clone(), fed2_id.clone()],
        true,
        true,
        fed3_policy.clone(),
    ).await?;
    
    println!("Federation routes configured.");

    // Demo: Store and retrieve data
    println!("\nDEMO: Storing and retrieving data across federations");
    println!("------------------------------------------------");
    
    // Store data in Federation 1
    println!("Storing data in Federation 1...");
    let data1 = b"This is sample data from Federation 1";
    storage_manager1.store_data(
        "data/shared/sample1.txt", 
        data1,
        Some(fed1_policy.clone()),
    ).await?;
    println!("Data stored in Federation 1.");
    
    // Federation 1 reads its own data
    println!("Federation 1 reading its own data...");
    let retrieved_data1 = match storage_manager1.retrieve_data("data/shared/sample1.txt").await {
        Ok(data) => {
            println!("Successfully retrieved data: {}", String::from_utf8_lossy(&data));
            data
        },
        Err(e) => {
            println!("Error retrieving data: {}", e);
            vec![]
        }
    };
    
    // Store data in Federation 2
    println!("Storing data in Federation 2...");
    let data2 = b"This is sample data from Federation 2";
    storage_manager2.store_data(
        "data/shared/sample2.txt", 
        data2,
        Some(fed2_policy.clone()),
    ).await?;
    println!("Data stored in Federation 2.");
    
    // Federation A tries to access data in Federation B (should succeed due to policy)
    println!("Federation 1 reading data from Federation 2...");
    let retrieved_data2 = match storage_manager1.retrieve_data("data/shared/sample2.txt").await {
        Ok(data) => {
            println!("Successfully retrieved data: {}", String::from_utf8_lossy(&data));
            data
        },
        Err(e) => {
            println!("Error retrieving data: {}", e);
            vec![]
        }
    };
    
    // Store data in Federation 3
    println!("Storing data in Federation 3...");
    let data3 = b"This is analytics data from Federation 3";
    storage_manager3.store_data(
        "data/analytics/results.txt", 
        data3,
        Some(fed3_policy.clone()),
    ).await?;
    println!("Data stored in Federation 3.");
    
    // Federation 2 tries to access data in Federation 3 (should succeed due to policy)
    println!("Federation 2 reading data from Federation 3...");
    let retrieved_data3 = match storage_manager2.retrieve_data("data/analytics/results.txt").await {
        Ok(data) => {
            println!("Successfully retrieved data: {}", String::from_utf8_lossy(&data));
            data
        },
        Err(e) => {
            println!("Error retrieving data: {}", e);
            vec![]
        }
    };
    
    // Federation 1 tries to access data in Federation 3 (should fail)
    println!("Federation 1 attempting to read data from Federation 3 (should fail)...");
    match storage_manager1.retrieve_data("data/analytics/results.txt").await {
        Ok(data) => {
            println!("Successfully retrieved data: {}", String::from_utf8_lossy(&data));
        },
        Err(e) => {
            println!("Error retrieving data (expected): {}", e);
        }
    };
    
    // Get federation storage stats
    println!("\nFederation Storage Statistics:");
    println!("------------------------------");
    
    let stats1 = storage_manager1.get_federation_storage_stats().await?;
    println!("Federation 1: {} peers, {:.2}% utilized, {} bytes available", 
             stats1.peer_count, stats1.utilization_percentage, stats1.available_space);
    
    let stats2 = storage_manager2.get_federation_storage_stats().await?;
    println!("Federation 2: {} peers, {:.2}% utilized, {} bytes available", 
             stats2.peer_count, stats2.utilization_percentage, stats2.available_space);
    
    let stats3 = storage_manager3.get_federation_storage_stats().await?;
    println!("Federation 3: {} peers, {:.2}% utilized, {} bytes available", 
             stats3.peer_count, stats3.utilization_percentage, stats3.available_space);
    
    println!("\nDemo completed successfully!");
    
    Ok(())
} 