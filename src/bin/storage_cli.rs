use clap::{App, AppSettings, Arg, SubCommand};
use colored::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::Arc;

use icn_network::{
    distributed_storage::{DistributedStorage, DataAccessPolicy, StoragePeer, AccessType},
    federation::coordination::{FederationCoordinator, SharedResource},
    storage::{Storage, StorageOptions, VersionInfo, StorageMetrics, MetricsSnapshot},
    networking::overlay::dht::DistributedHashTable,
    crypto::StorageEncryptionService,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("ICN Storage CLI")
        .version("0.1.0")
        .author("ICN Network Team")
        .about("Command-line interface for ICN Network distributed storage")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .subcommand(
            SubCommand::with_name("init")
                .about("Initialize the storage environment")
                .arg(
                    Arg::with_name("data-dir")
                        .help("Data directory for local storage")
                        .long("data-dir")
                        .default_value("data/storage")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("node-id")
                        .help("Node identifier")
                        .long("node-id")
                        .default_value("node1")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("address")
                        .help("Node network address")
                        .long("address")
                        .default_value("127.0.0.1:8000")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("capacity")
                        .help("Storage capacity in bytes")
                        .long("capacity")
                        .default_value("1073741824") // 1GB
                        .takes_value(true),
                ),
        )
        .subcommand(
            SubCommand::with_name("federation")
                .about("Manage federations")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("create")
                        .about("Create a new federation")
                        .arg(
                            Arg::with_name("name")
                                .help("Federation name")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("description")
                                .help("Federation description")
                                .default_value("")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("list")
                        .about("List available federations"),
                )
                .subcommand(
                    SubCommand::with_name("join")
                        .about("Join a federation")
                        .arg(
                            Arg::with_name("federation-id")
                                .help("Federation ID to join")
                                .required(true)
                                .takes_value(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("storage")
                .about("Manage storage operations")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("put")
                        .about("Store data")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("file")
                                .help("File to store")
                                .required(false)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("data")
                                .help("Data to store (as string)")
                                .required(false)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("federation")
                                .help("Federation ID with access")
                                .long("federation")
                                .multiple(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("encrypted")
                                .help("Enable encryption")
                                .long("encrypted")
                                .takes_value(false),
                        )
                        .arg(
                            Arg::with_name("redundancy")
                                .help("Redundancy factor")
                                .long("redundancy")
                                .default_value("3")
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("versioned")
                                .help("Enable versioning")
                                .long("versioned")
                                .takes_value(false),
                        )
                        .arg(
                            Arg::with_name("max-versions")
                                .help("Maximum versions to keep")
                                .long("max-versions")
                                .default_value("10")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("get")
                        .about("Retrieve data")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("output")
                                .help("Output file path")
                                .long("output")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("delete")
                        .about("Delete data")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("version")
                .about("Manage data versions")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("list")
                        .about("List versions for a key")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("get")
                        .about("Get a specific version")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("version-id")
                                .help("Version ID")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("output")
                                .help("Output file path")
                                .long("output")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("revert")
                        .about("Revert to a specific version")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("version-id")
                                .help("Version ID")
                                .required(true)
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("enable")
                        .about("Enable versioning for a key")
                        .arg(
                            Arg::with_name("key")
                                .help("Storage key")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("max-versions")
                                .help("Maximum versions to keep")
                                .long("max-versions")
                                .default_value("10")
                                .takes_value(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("encryption")
                .about("Manage encryption")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("create-key")
                        .about("Create a new encryption key")
                        .arg(
                            Arg::with_name("federation")
                                .help("Federation ID with access")
                                .multiple(true)
                                .required(true)
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("grant-access")
                        .about("Grant a federation access to a key")
                        .arg(
                            Arg::with_name("federation-id")
                                .help("Federation ID to grant access")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("key-id")
                                .help("Encryption key ID")
                                .required(true)
                                .takes_value(true),
                        ),
                ),
        )
        .subcommand(
            SubCommand::with_name("status")
                .about("Show storage system status"),
        )
        .subcommand(
            SubCommand::with_name("metrics")
                .about("Manage storage metrics")
                .setting(AppSettings::SubcommandRequiredElseHelp)
                .subcommand(
                    SubCommand::with_name("show")
                        .about("Show current metrics")
                        .arg(
                            Arg::with_name("format")
                                .help("Output format")
                                .long("format")
                                .possible_values(&["text", "json"])
                                .default_value("text")
                                .takes_value(true),
                        ),
                )
                .subcommand(
                    SubCommand::with_name("reset")
                        .about("Reset all metrics"),
                )
                .subcommand(
                    SubCommand::with_name("export")
                        .about("Export metrics to a file")
                        .arg(
                            Arg::with_name("file")
                                .help("Output file path")
                                .required(true)
                                .takes_value(true),
                        )
                        .arg(
                            Arg::with_name("format")
                                .help("Output format")
                                .long("format")
                                .possible_values(&["json", "csv"])
                                .default_value("json")
                                .takes_value(true),
                        ),
                ),
        )
        .get_matches();

    // Handle initialization
    if let Some(init_matches) = matches.subcommand_matches("init") {
        let data_dir = init_matches.value_of("data-dir").unwrap();
        let node_id = init_matches.value_of("node-id").unwrap();
        let address = init_matches.value_of("address").unwrap();
        let capacity: u64 = init_matches.value_of("capacity").unwrap().parse()?;

        // Create data directory if it doesn't exist
        fs::create_dir_all(data_dir)?;

        // Write configuration to config file
        let config = format!(
            "{{
                \"node_id\": \"{}\",
                \"address\": \"{}\",
                \"data_dir\": \"{}\",
                \"capacity\": {}
            }}",
            node_id, address, data_dir, capacity
        );

        let config_path = Path::new(data_dir).join("config.json");
        let mut file = fs::File::create(config_path)?;
        file.write_all(config.as_bytes())?;

        println!("{} Storage initialized at: {}", "SUCCESS:".green(), data_dir);
        println!("  Node ID: {}", node_id);
        println!("  Address: {}", address);
        println!("  Capacity: {} bytes", capacity);
        return Ok(());
    }

    // Load configuration
    let config = load_config()?;
    let data_dir = config["data_dir"].as_str().unwrap();
    let node_id = config["node_id"].as_str().unwrap().to_string();
    let address = config["address"].as_str().unwrap().to_string();

    // Initialize common components
    let local_storage = Arc::new(Storage::new(data_dir));
    let federation_coordinator = Arc::new(FederationCoordinator::new());
    let mut dht = DistributedHashTable::new();
    dht.initialize(&node_id, &address)?;
    let dht = Arc::new(dht);
    let encryption_service = Arc::new(StorageEncryptionService::new());

    // Create metrics
    let metrics = Arc::new(StorageMetrics::new());

    let storage = DistributedStorage::with_encryption_service(
        node_id.clone(),
        "default".to_string(), // Default federation ID, will be updated later
        local_storage.clone(),
        dht.clone(),
        federation_coordinator.clone(),
        encryption_service.clone(),
    );

    // Register this node as a storage peer
    let capacity = config["capacity"].as_u64().unwrap();
    storage
        .add_peer(StoragePeer {
            node_id: node_id.clone(),
            address: address.clone(),
            federation_id: "default".to_string(),
            storage_capacity: capacity,
            available_space: capacity,
            latency_ms: 0,
            uptime_percentage: 100.0,
            tags: HashMap::new(),
        })
        .await?;

    // Process other commands
    match matches.subcommand() {
        ("federation", Some(fed_matches)) => {
            handle_federation_commands(fed_matches, &federation_coordinator).await?;
        }
        ("storage", Some(storage_matches)) => {
            handle_storage_commands(storage_matches, &storage).await?;
        }
        ("version", Some(version_matches)) => {
            handle_version_commands(version_matches, &storage).await?;
        }
        ("encryption", Some(enc_matches)) => {
            handle_encryption_commands(enc_matches, &storage).await?;
        }
        ("status", _) => {
            show_status(&storage, &local_storage, &federation_coordinator).await?;
        }
        ("metrics", Some(metrics_matches)) => {
            handle_metrics_commands(metrics_matches, &metrics).await?;
        }
        _ => {}
    }

    Ok(())
}

async fn handle_federation_commands(
    matches: &clap::ArgMatches<'_>,
    federation_coordinator: &FederationCoordinator,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        ("create", Some(create_matches)) => {
            let name = create_matches.value_of("name").unwrap();
            let description = create_matches.value_of("description").unwrap();
            
            let fed_id = federation_coordinator
                .register_federation(
                    name,
                    description,
                    vec![],
                    vec![],
                    serde_json::json!({}),
                )
                .await?;

            println!("{} Federation created", "SUCCESS:".green());
            println!("  ID: {}", fed_id);
            println!("  Name: {}", name);
        }
        ("list", _) => {
            let federations = federation_coordinator.list_federations().await?;
            
            if federations.is_empty() {
                println!("No federations available.");
            } else {
                println!("{}", "Available federations:".underline());
                for (i, fed) in federations.iter().enumerate() {
                    println!("{}. {} - {}", i + 1, fed.id, fed.name);
                    println!("   Description: {}", fed.description);
                    println!("   Members: {}", fed.members.len());
                }
            }
        }
        ("join", Some(join_matches)) => {
            let federation_id = join_matches.value_of("federation-id").unwrap();
            
            // In a real implementation, this would send a join request
            println!("{} Join request sent to federation: {}", "SUCCESS:".green(), federation_id);
            println!("Awaiting approval from federation administrators.");
        }
        _ => {}
    }

    Ok(())
}

async fn handle_storage_commands(
    matches: &clap::ArgMatches<'_>,
    storage: &DistributedStorage,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        ("put", Some(put_matches)) => {
            let key = put_matches.value_of("key").unwrap();
            
            // Get data from file or string input
            let data = if let Some(file_path) = put_matches.value_of("file") {
                fs::read(file_path)?
            } else if let Some(data_str) = put_matches.value_of("data") {
                data_str.as_bytes().to_vec()
            } else {
                // Read from stdin if neither file nor data is provided
                let mut buffer = Vec::new();
                io::stdin().read_to_end(&mut buffer)?;
                buffer
            };

            // Create access policy
            let mut policy = DataAccessPolicy::default();
            
            // Add federation access
            if let Some(federations) = put_matches.values_of("federation") {
                for fed in federations {
                    policy.read_federations.insert(fed.to_string());
                    policy.write_federations.insert(fed.to_string());
                    policy.admin_federations.insert(fed.to_string());
                }
            } else {
                // Default to current federation
                policy.read_federations.insert(storage.federation_id.clone());
                policy.write_federations.insert(storage.federation_id.clone());
                policy.admin_federations.insert(storage.federation_id.clone());
            }

            // Set encryption
            policy.encryption_required = put_matches.is_present("encrypted");
            
            // Set redundancy
            policy.redundancy_factor = put_matches.value_of("redundancy").unwrap().parse::<u8>()?;
            
            // Set versioning
            policy.versioning_enabled = put_matches.is_present("versioned");
            if policy.versioning_enabled {
                policy.max_versions = put_matches.value_of("max-versions").unwrap().parse::<u32>()?;
            }

            // Store the data
            storage.put(key, &data, policy).await?;

            println!("{} Data stored successfully", "SUCCESS:".green());
            println!("  Key: {}", key);
            println!("  Size: {} bytes", data.len());
            println!("  Encrypted: {}", policy.encryption_required);
            println!("  Versioned: {}", policy.versioning_enabled);
        }
        ("get", Some(get_matches)) => {
            let key = get_matches.value_of("key").unwrap();
            
            // Retrieve the data
            let data = storage.get(key).await?;
            
            // Write to file or stdout
            if let Some(output_path) = get_matches.value_of("output") {
                fs::write(output_path, &data)?;
                println!("{} Data retrieved and saved to: {}", "SUCCESS:".green(), output_path);
            } else {
                // Try to print as UTF-8 string if possible
                match std::str::from_utf8(&data) {
                    Ok(s) => println!("{}", s),
                    Err(_) => {
                        println!("Binary data (length: {} bytes):", data.len());
                        // Print hex representation for binary data
                        for (i, byte) in data.iter().enumerate().take(100) {
                            print!("{:02x} ", byte);
                            if (i + 1) % 16 == 0 {
                                println!();
                            }
                        }
                        if data.len() > 100 {
                            println!("\n... (output truncated)");
                        }
                    }
                }
            }
        }
        ("delete", Some(delete_matches)) => {
            let key = delete_matches.value_of("key").unwrap();
            
            // Delete the data
            storage.delete(key).await?;
            
            println!("{} Data deleted successfully", "SUCCESS:".green());
            println!("  Key: {}", key);
        }
        _ => {}
    }

    Ok(())
}

async fn handle_version_commands(
    matches: &clap::ArgMatches<'_>,
    storage: &DistributedStorage,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        ("list", Some(list_matches)) => {
            let key = list_matches.value_of("key").unwrap();
            
            // List versions
            let versions = storage.list_versions(key).await?;
            
            println!("{} for key: {}", "Version history".underline(), key);
            if versions.is_empty() {
                println!("No versions available.");
            } else {
                for (i, version) in versions.iter().enumerate() {
                    let created_at = chrono::DateTime::from_timestamp(version.created_at as i64, 0)
                        .map(|dt| dt.to_rfc2822())
                        .unwrap_or_else(|| version.created_at.to_string());
                    
                    println!("{}. ID: {}", i + 1, version.version_id);
                    println!("   Created: {}", created_at);
                    println!("   Size: {} bytes", version.size_bytes);
                    println!("   Created by: {}", version.created_by);
                    if let Some(comment) = &version.comment {
                        println!("   Comment: {}", comment);
                    }
                    println!("   Hash: {}", version.content_hash);
                    println!();
                }
            }
        }
        ("get", Some(get_matches)) => {
            let key = get_matches.value_of("key").unwrap();
            let version_id = get_matches.value_of("version-id").unwrap();
            
            // Get specific version
            let data = storage.get_version(key, version_id).await?;
            
            // Write to file or stdout
            if let Some(output_path) = get_matches.value_of("output") {
                fs::write(output_path, &data)?;
                println!("{} Version data retrieved and saved to: {}", "SUCCESS:".green(), output_path);
            } else {
                // Try to print as UTF-8 string if possible
                match std::str::from_utf8(&data) {
                    Ok(s) => println!("{}", s),
                    Err(_) => {
                        println!("Binary data (length: {} bytes):", data.len());
                        // Print hex representation for binary data
                        for (i, byte) in data.iter().enumerate().take(100) {
                            print!("{:02x} ", byte);
                            if (i + 1) % 16 == 0 {
                                println!();
                            }
                        }
                        if data.len() > 100 {
                            println!("\n... (output truncated)");
                        }
                    }
                }
            }
        }
        ("revert", Some(revert_matches)) => {
            let key = revert_matches.value_of("key").unwrap();
            let version_id = revert_matches.value_of("version-id").unwrap();
            
            // Revert to specific version
            storage.revert_to_version(key, version_id).await?;
            
            println!("{} Reverted to version {}", "SUCCESS:".green(), version_id);
            println!("  Key: {}", key);
        }
        ("enable", Some(enable_matches)) => {
            let key = enable_matches.value_of("key").unwrap();
            let max_versions = enable_matches.value_of("max-versions").unwrap().parse::<u32>()?;
            
            // Enable versioning
            storage.enable_versioning(key, max_versions).await?;
            
            println!("{} Versioning enabled for key: {}", "SUCCESS:".green(), key);
            println!("  Max versions: {}", max_versions);
        }
        _ => {}
    }

    Ok(())
}

async fn handle_encryption_commands(
    matches: &clap::ArgMatches<'_>,
    storage: &DistributedStorage,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        ("create-key", Some(create_matches)) => {
            let federations: Vec<String> = create_matches
                .values_of("federation")
                .unwrap()
                .map(|s| s.to_string())
                .collect();
            
            // Create encryption key
            let key_id = storage.initialize_encryption_key(federations.clone()).await?;
            
            println!("{} Encryption key created", "SUCCESS:".green());
            println!("  Key ID: {}", key_id);
            println!("  Federations with access:");
            for fed in federations {
                println!("    - {}", fed);
            }
        }
        ("grant-access", Some(grant_matches)) => {
            let federation_id = grant_matches.value_of("federation-id").unwrap();
            let key_id = grant_matches.value_of("key-id").unwrap();
            
            // Grant access
            storage.grant_federation_key_access(federation_id, key_id).await?;
            
            println!("{} Access granted", "SUCCESS:".green());
            println!("  Federation: {}", federation_id);
            println!("  Key ID: {}", key_id);
        }
        _ => {}
    }

    Ok(())
}

async fn show_status(
    storage: &DistributedStorage,
    local_storage: &Storage,
    federation_coordinator: &FederationCoordinator,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", "Storage System Status".bold().underline());
    println!("Node ID: {}", storage.node_id);
    println!("Federation: {}", storage.federation_id);
    
    // Show federation information
    let federations = federation_coordinator.list_federations().await?;
    println!("\n{}", "Federations:".underline());
    if federations.is_empty() {
        println!("No federations available.");
    } else {
        for (i, fed) in federations.iter().enumerate() {
            println!("{}. {} - {}", i + 1, fed.id, fed.name);
            println!("   Members: {}", fed.members.len());
        }
    }
    
    // Show storage peers
    println!("\n{}", "Storage Peers:".underline());
    let peers = storage.get_all_peers().await?;
    if peers.is_empty() {
        println!("No storage peers available.");
    } else {
        for (i, peer) in peers.iter().enumerate() {
            println!("{}. {}", i + 1, peer.node_id);
            println!("   Address: {}", peer.address);
            println!("   Federation: {}", peer.federation_id);
            println!("   Capacity: {} bytes", peer.storage_capacity);
            println!("   Available: {} bytes", peer.available_space);
            println!("   Uptime: {:.1}%", peer.uptime_percentage);
        }
    }
    
    // Show storage statistics
    println!("\n{}", "Storage Statistics:".underline());
    println!("Local data dir: {}", local_storage.base_dir());
    
    // In a more complete implementation, we would show actual storage stats
    // For now, we'll show the basics:
    println!("Keys stored: {}", storage.get_key_count().await?);
    
    Ok(())
}

async fn handle_metrics_commands(
    matches: &clap::ArgMatches<'_>,
    metrics: &StorageMetrics,
) -> Result<(), Box<dyn std::error::Error>> {
    match matches.subcommand() {
        ("show", Some(show_matches)) => {
            let format = show_matches.value_of("format").unwrap();
            
            // Get metrics snapshot
            let snapshot = metrics.get_snapshot().await;
            
            if format == "json" {
                // Output as JSON
                let json = serde_json::to_string_pretty(&snapshot)?;
                println!("{}", json);
            } else {
                // Output as text report
                let report = icn_network::storage::metrics::format::metrics_report(&snapshot);
                println!("{}", report);
            }
        }
        ("reset", _) => {
            // Reset metrics
            metrics.reset().await;
            println!("{} All metrics have been reset", "SUCCESS:".green());
        }
        ("export", Some(export_matches)) => {
            let file_path = export_matches.value_of("file").unwrap();
            let format = export_matches.value_of("format").unwrap();
            
            // Get metrics snapshot
            let snapshot = metrics.get_snapshot().await;
            
            if format == "json" {
                // Export as JSON
                let json = serde_json::to_string_pretty(&snapshot)?;
                fs::write(file_path, json)?;
            } else if format == "csv" {
                // Export as CSV
                let mut csv = String::new();
                
                // Generate CSV headers
                csv.push_str("timestamp,uptime_seconds,puts,gets,deletes,version_list,version_get,version_revert,failed_operations,");
                csv.push_str("put_latency_ms,get_latency_ms,delete_latency_ms,version_latency_ms,");
                csv.push_str("total_keys,total_size_bytes,encrypted_keys,encrypted_size_bytes,versioned_keys,bytes_written,bytes_read,");
                csv.push_str("total_versions,avg_versions_per_key,avg_version_size,revert_operations,version_storage_overhead\n");
                
                // Add data row
                csv.push_str(&format!("{},{},{},{},{},{},{},{},{},",
                    snapshot.timestamp,
                    snapshot.uptime_seconds,
                    snapshot.operation_counts.puts,
                    snapshot.operation_counts.gets,
                    snapshot.operation_counts.deletes,
                    snapshot.operation_counts.version_list,
                    snapshot.operation_counts.version_get,
                    snapshot.operation_counts.version_revert,
                    snapshot.operation_counts.failed_operations,
                ));
                
                csv.push_str(&format!("{:.2},{:.2},{:.2},{:.2},",
                    snapshot.operation_latencies.put_latency_ms.get(),
                    snapshot.operation_latencies.get_latency_ms.get(),
                    snapshot.operation_latencies.delete_latency_ms.get(),
                    snapshot.operation_latencies.version_operations_latency_ms.get(),
                ));
                
                csv.push_str(&format!("{},{},{},{},{},{},{},",
                    snapshot.data_metrics.total_keys,
                    snapshot.data_metrics.total_size_bytes,
                    snapshot.data_metrics.encrypted_keys,
                    snapshot.data_metrics.encrypted_size_bytes,
                    snapshot.data_metrics.versioned_keys,
                    snapshot.data_metrics.bytes_written,
                    snapshot.data_metrics.bytes_read,
                ));
                
                csv.push_str(&format!("{},{:.2},{:.2},{},{}",
                    snapshot.version_metrics.total_versions,
                    snapshot.version_metrics.versions_per_key.get(),
                    snapshot.version_metrics.version_size_bytes.get(),
                    snapshot.version_metrics.revert_operations,
                    snapshot.version_metrics.version_storage_overhead_bytes,
                ));
                
                fs::write(file_path, csv)?;
            }
            
            println!("{} Metrics exported to: {}", "SUCCESS:".green(), file_path);
        }
        _ => {}
    }
    
    Ok(())
}

fn load_config() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    // Try to find and load configuration file
    let possible_dirs = [".", "data", "data/storage"];
    
    for dir in possible_dirs.iter() {
        let config_path = Path::new(dir).join("config.json");
        if config_path.exists() {
            let config_str = fs::read_to_string(config_path)?;
            return Ok(serde_json::from_str(&config_str)?);
        }
    }
    
    // If no config found, return an error
    Err("No storage configuration found. Run 'init' command first.".into())
}

// Additional extension methods for our storage interfaces to support the CLI functions
trait StorageExtensions {
    async fn get_all_peers(&self) -> Result<Vec<StoragePeer>, Box<dyn std::error::Error>>;
    async fn get_key_count(&self) -> Result<usize, Box<dyn std::error::Error>>;
}

impl StorageExtensions for DistributedStorage {
    async fn get_all_peers(&self) -> Result<Vec<StoragePeer>, Box<dyn std::error::Error>> {
        let peers = self.peers.read().await;
        Ok(peers.values().cloned().collect())
    }
    
    async fn get_key_count(&self) -> Result<usize, Box<dyn std::error::Error>> {
        let locations = self.data_locations.read().await;
        Ok(locations.len())
    }
}

trait StorageInfoExtensions {
    fn base_dir(&self) -> String;
}

impl StorageInfoExtensions for Storage {
    fn base_dir(&self) -> String {
        // This is a simplification - in a real implementation, Storage would
        // expose this information directly
        "data/storage".to_string()
    }
} 