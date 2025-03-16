use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use icn_network::{
    distributed_storage::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType},
    federation::coordination::{FederationCoordinator},
    storage::{Storage, StorageOptions, VersionInfo},
    networking::overlay::dht::DistributedHashTable,
    crypto::StorageEncryptionService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("ICN Versioned Storage Demo");
    println!("=========================\n");

    // Setup storage directories
    println!("Setting up storage environment...");
    let base_dir = "data/versioned_demo";
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
        "For versioning demonstration",
        vec!["node1".to_string()],
        vec![],
        serde_json::json!({"purpose": "demo"}),
    ).await?;
    println!("Registered federation with ID: {}", fed_id);

    // Create distributed storage with encryption
    println!("Initializing distributed storage...");
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

    // Demo 1: Create and update versioned data
    println!("\nDEMO 1: Creating and updating versioned data");
    println!("------------------------------------------");

    // Create data and policy with versioning enabled
    let demo_data = b"Version 1 of the document";
    
    let mut policy = DataAccessPolicy::default();
    policy.read_federations.insert(fed_id.clone());
    policy.write_federations.insert(fed_id.clone());
    policy.admin_federations.insert(fed_id.clone());
    policy.versioning_enabled = true;
    policy.max_versions = 5;

    // Store the initial version
    println!("Storing initial version of data...");
    storage.put(
        "demo/versioned_doc.txt",
        demo_data,
        policy.clone(),
    ).await?;
    println!("Initial version stored successfully");

    // Retrieve the data
    println!("Retrieving the current version...");
    let retrieved_data = storage.get("demo/versioned_doc.txt").await?;
    println!("Current version content: '{}'", String::from_utf8_lossy(&retrieved_data));

    // Update with a new version
    println!("\nUpdating the document with a new version...");
    let demo_data_v2 = b"Version 2 of the document with more content";
    storage.put(
        "demo/versioned_doc.txt",
        demo_data_v2,
        policy.clone(),
    ).await?;
    println!("Version 2 stored successfully");
    
    // List versions
    println!("\nListing all versions:");
    let versions = storage.list_versions("demo/versioned_doc.txt").await?;
    for (i, version) in versions.iter().enumerate() {
        println!("  Version {}: ID={}, Created at={}, Size={} bytes",
                 i + 1, version.version_id, version.created_at, version.size_bytes);
    }

    // Create a third version
    println!("\nUpdating with a third version...");
    let demo_data_v3 = b"Version 3 with even more modifications and content";
    storage.put(
        "demo/versioned_doc.txt",
        demo_data_v3,
        policy.clone(),
    ).await?;
    println!("Version 3 stored successfully");

    // Verify we have the latest version
    println!("\nRetrieving the current version (should be version 3):");
    let current_data = storage.get("demo/versioned_doc.txt").await?;
    println!("Current version content: '{}'", String::from_utf8_lossy(&current_data));

    // Demo 2: Accessing historical versions
    println!("\nDEMO 2: Accessing historical versions");
    println!("-----------------------------------");

    // List all versions
    println!("Listing all versions:");
    let versions = storage.list_versions("demo/versioned_doc.txt").await?;
    
    // Get the second version ID (middle version)
    let version2_id = &versions[1].version_id;
    
    // Retrieve version 2
    println!("\nRetrieving version 2 by ID ({})...", version2_id);
    let version2_data = storage.get_version("demo/versioned_doc.txt", version2_id).await?;
    println!("Version 2 content: '{}'", String::from_utf8_lossy(&version2_data));

    // Get the first version ID (oldest version)
    let version1_id = &versions[2].version_id;
    
    // Retrieve version 1
    println!("\nRetrieving version 1 by ID ({})...", version1_id);
    let version1_data = storage.get_version("demo/versioned_doc.txt", version1_id).await?;
    println!("Version 1 content: '{}'", String::from_utf8_lossy(&version1_data));

    // Demo 3: Reverting to a previous version
    println!("\nDEMO 3: Reverting to a previous version");
    println!("--------------------------------------");

    // Revert to version 1
    println!("Reverting to version 1...");
    storage.revert_to_version("demo/versioned_doc.txt", version1_id).await?;
    
    // Verify we now have version 1 as the current version
    println!("\nRetrieving the current version (should now be version 1):");
    let reverted_data = storage.get("demo/versioned_doc.txt").await?;
    println!("Current version content: '{}'", String::from_utf8_lossy(&reverted_data));
    
    // Verify it matches the original version 1
    if reverted_data == demo_data.to_vec() {
        println!("SUCCESS: Current version correctly reverted to version 1");
    } else {
        println!("ERROR: Reversion did not work correctly");
    }

    // Demo 4: Enabling versioning for existing data
    println!("\nDEMO 4: Enabling versioning for existing data");
    println!("-------------------------------------------");

    // Create unversioned data
    let unversioned_data = b"This is unversioned data";
    let mut unversioned_policy = DataAccessPolicy::default();
    unversioned_policy.read_federations.insert(fed_id.clone());
    unversioned_policy.write_federations.insert(fed_id.clone());
    unversioned_policy.admin_federations.insert(fed_id.clone());
    unversioned_policy.versioning_enabled = false;

    // Store unversioned data
    println!("Storing unversioned data...");
    storage.put(
        "demo/unversioned_doc.txt",
        unversioned_data,
        unversioned_policy,
    ).await?;
    println!("Unversioned data stored successfully");

    // Enable versioning for this data
    println!("\nEnabling versioning for the existing data...");
    storage.enable_versioning("demo/unversioned_doc.txt", 5).await?;
    println!("Versioning enabled successfully");

    // Update the data to create a new version
    println!("\nUpdating the data to create a new version...");
    let updated_data = b"This data now has an updated version";
    
    let mut updated_policy = DataAccessPolicy::default();
    updated_policy.read_federations.insert(fed_id.clone());
    updated_policy.write_federations.insert(fed_id.clone());
    updated_policy.admin_federations.insert(fed_id.clone());
    updated_policy.versioning_enabled = true;
    updated_policy.max_versions = 5;
    
    storage.put(
        "demo/unversioned_doc.txt",
        updated_data,
        updated_policy,
    ).await?;
    println!("Update successful");

    // List versions
    println!("\nListing versions for the previously unversioned data:");
    let versions = storage.list_versions("demo/unversioned_doc.txt").await?;
    for (i, version) in versions.iter().enumerate() {
        println!("  Version {}: ID={}, Created at={}, Size={} bytes",
                 i + 1, version.version_id, version.created_at, version.size_bytes);
    }

    // Verify we can access both versions
    let current_version = storage.get("demo/unversioned_doc.txt").await?;
    println!("\nCurrent version: '{}'", String::from_utf8_lossy(&current_version));
    
    let initial_version_id = &versions[1].version_id;
    let initial_version = storage.get_version("demo/unversioned_doc.txt", initial_version_id).await?;
    println!("Initial version: '{}'", String::from_utf8_lossy(&initial_version));

    println!("\nVersioned storage demo completed successfully!");
    
    Ok(())
} 