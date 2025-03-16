use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use icn_network::{
    distributed_storage::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType},
    federation::coordination::{FederationCoordinator},
    storage::{Storage, StorageOptions},
    networking::overlay::dht::DistributedHashTable,
    crypto::StorageEncryptionService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ICN Encrypted Storage Demo");
    println!("=========================\n");

    // Setup storage directories
    println!("Setting up storage environment...");
    let base_dir = "data/encrypted_demo";
    std::fs::create_dir_all(base_dir)?;
    println!("Storage directory created at: {}", base_dir);

    // Create local storage instance
    let local_storage = Arc::new(Storage::new(base_dir));

    // Create federation coordinator and DHT
    let federation_coordinator = Arc::new(FederationCoordinator::new());
    let mut dht = DistributedHashTable::new();
    dht.initialize(&"node1".to_string(), &"addr1".into())?;
    let dht = Arc::new(dht);

    // Create encryption service
    let encryption_service = Arc::new(StorageEncryptionService::new());

    // Register federation
    println!("Registering federation...");
    let fed_id = federation_coordinator.register_federation(
        "Demo Federation",
        "For encryption demonstration",
        vec!["node1".to_string()],
        vec![],
        serde_json::json!({"purpose": "demo"}),
    ).await?;
    println!("Registered federation with ID: {}", fed_id);

    // Create distributed storage with encryption
    println!("Initializing distributed storage with encryption...");
    let storage = DistributedStorage::with_encryption_service(
        "node1".to_string(),
        fed_id.clone(),
        local_storage,
        dht,
        federation_coordinator.clone(),
        encryption_service,
    );

    // Register a storage peer (ourselves)
    storage.add_peer(StoragePeer {
        node_id: "node1".to_string(),
        address: "127.0.0.1:8000".to_string(),
        federation_id: fed_id.clone(),
        storage_capacity: 1024 * 1024 * 1024, // 1GB
        available_space: 1024 * 1024 * 1024,  // 1GB
        latency_ms: 0,
        uptime_percentage: 100.0,
        tags: HashMap::new(),
    }).await?;

    // Initialize encryption key
    println!("Initializing encryption key for the federation...");
    let key_id = storage.initialize_encryption_key(vec![fed_id.clone()]).await?;
    println!("Generated encryption key with ID: {}", key_id);

    // Demo 1: Store and retrieve encrypted data
    println!("\nDEMO 1: Storing and retrieving encrypted data");
    println!("------------------------------------------");

    // Create data and policy
    let demo_data = b"This is secure data that will be encrypted";
    let mut policy = DataAccessPolicy::default();
    policy.read_federations.insert(fed_id.clone());
    policy.write_federations.insert(fed_id.clone());
    policy.admin_federations.insert(fed_id.clone());
    policy.encryption_required = true;

    // Store the data
    println!("Storing encrypted data...");
    storage.put(
        "demo/encrypted.txt",
        demo_data,
        policy,
    ).await?;
    println!("Data stored successfully with encryption");

    // Retrieve the data
    println!("Retrieving and decrypting data...");
    let retrieved_data = storage.get("demo/encrypted.txt").await?;
    
    // Verify the data matches
    if retrieved_data == demo_data {
        println!("SUCCESS: Retrieved data matches original data");
        println!("Original:  {}", String::from_utf8_lossy(demo_data));
        println!("Retrieved: {}", String::from_utf8_lossy(&retrieved_data));
    } else {
        println!("ERROR: Retrieved data does not match original data!");
        println!("Original:  {}", String::from_utf8_lossy(demo_data));
        println!("Retrieved: {}", String::from_utf8_lossy(&retrieved_data));
    }

    // Demo 2: Store unencrypted data for comparison
    println!("\nDEMO 2: Storing and retrieving unencrypted data");
    println!("--------------------------------------------");

    // Create data and policy without encryption
    let demo_data2 = b"This is non-sensitive data that will not be encrypted";
    let mut policy2 = DataAccessPolicy::default();
    policy2.read_federations.insert(fed_id.clone());
    policy2.write_federations.insert(fed_id.clone());
    policy2.admin_federations.insert(fed_id.clone());
    policy2.encryption_required = false;

    // Store the data
    println!("Storing unencrypted data...");
    storage.put(
        "demo/unencrypted.txt",
        demo_data2,
        policy2,
    ).await?;
    println!("Data stored successfully without encryption");

    // Retrieve the data
    println!("Retrieving data...");
    let retrieved_data2 = storage.get("demo/unencrypted.txt").await?;
    
    // Verify the data matches
    if retrieved_data2 == demo_data2 {
        println!("SUCCESS: Retrieved data matches original data");
        println!("Original:  {}", String::from_utf8_lossy(demo_data2));
        println!("Retrieved: {}", String::from_utf8_lossy(&retrieved_data2));
    } else {
        println!("ERROR: Retrieved data does not match original data!");
        println!("Original:  {}", String::from_utf8_lossy(demo_data2));
        println!("Retrieved: {}", String::from_utf8_lossy(&retrieved_data2));
    }

    // Demo 3: Access control with encryption keys
    println!("\nDEMO 3: Federation-based encryption key access control");
    println!("--------------------------------------------------");

    // Create a new federation
    println!("Creating a second federation without encryption key access...");
    let fed2_id = federation_coordinator.register_federation(
        "Second Federation",
        "For testing encryption access",
        vec!["node2".to_string()],
        vec![],
        serde_json::json!({"purpose": "demo"}),
    ).await?;
    println!("Registered second federation with ID: {}", fed2_id);

    // Create a new storage instance for the second federation
    let storage2 = DistributedStorage::with_encryption_service(
        "node2".to_string(),
        fed2_id.clone(),
        Arc::new(Storage::new(format!("{}/fed2", base_dir).as_str())),
        dht.clone(),
        federation_coordinator.clone(),
        storage.encryption_service.clone(),
    );

    // Register a storage peer for the second federation
    storage2.add_peer(StoragePeer {
        node_id: "node2".to_string(),
        address: "127.0.0.1:8001".to_string(),
        federation_id: fed2_id.clone(),
        storage_capacity: 1024 * 1024 * 1024, // 1GB
        available_space: 1024 * 1024 * 1024,  // 1GB
        latency_ms: 0,
        uptime_percentage: 100.0,
        tags: HashMap::new(),
    }).await?;

    // Set up an agreement between federations
    println!("Setting up federation agreement...");
    let agreement_id = federation_coordinator.propose_agreement(
        &fed_id,
        &fed2_id,
        vec![],
        vec![],
        86400, // 1 day
    ).await?;
    
    federation_coordinator.activate_agreement(&agreement_id, &fed_id).await?;
    federation_coordinator.activate_agreement(&agreement_id, &fed2_id).await?;
    println!("Federation agreement activated with ID: {}", agreement_id);

    // Create cross-federation data
    let cross_fed_data = b"This data should be accessible to both federations";
    let mut cross_fed_policy = DataAccessPolicy::default();
    cross_fed_policy.read_federations.insert(fed_id.clone());
    cross_fed_policy.read_federations.insert(fed2_id.clone());
    cross_fed_policy.write_federations.insert(fed_id.clone());
    cross_fed_policy.admin_federations.insert(fed_id.clone());
    cross_fed_policy.encryption_required = true;

    // Store data with cross-federation access
    println!("Storing cross-federation encrypted data...");
    storage.put(
        "demo/cross_federation.txt",
        cross_fed_data,
        cross_fed_policy,
    ).await?;
    println!("Cross-federation data stored successfully");

    // Try to access from second federation (should fail without key access)
    println!("Attempting to retrieve encrypted data from second federation (should fail)...");
    match storage2.get("demo/cross_federation.txt").await {
        Ok(_) => {
            println!("ERROR: Second federation shouldn't have access yet!");
        },
        Err(e) => {
            println!("SUCCESS: Access denied as expected: {}", e);
        }
    }

    // Grant encryption key access to the second federation
    println!("Granting encryption key access to second federation...");
    storage.grant_federation_key_access(&fed2_id, &key_id).await?;
    println!("Encryption key access granted to second federation");

    // Try to access again from second federation (should succeed now)
    println!("Attempting to retrieve encrypted data from second federation (should succeed)...");
    match storage2.get("demo/cross_federation.txt").await {
        Ok(data) => {
            if data == cross_fed_data {
                println!("SUCCESS: Second federation can now access the encrypted data");
                println!("Retrieved: {}", String::from_utf8_lossy(&data));
            } else {
                println!("ERROR: Retrieved data does not match original data!");
            }
        },
        Err(e) => {
            println!("ERROR: Access still failed: {}", e);
        }
    }

    println!("\nEncrypted storage demo completed successfully!");
    
    Ok(())
} 