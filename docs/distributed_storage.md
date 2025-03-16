# Distributed Storage System

The ICN Network provides a federation-aware distributed storage system that enables secure data storage across cooperative nodes with flexible access control policies.

## Architecture Overview

The distributed storage system consists of the following key components:

1. **Distributed Storage** (`DistributedStorage`): Core component that handles data storage with proximity-aware peer selection and federation-based access control.

2. **Federation Storage Router** (`FederationStorageRouter`): Routes data storage and retrieval requests across multiple federations based on key prefixes and access policies.

3. **Federation Storage Manager** (`FederationStorageManager`): Manages storage peers within a federation and provides a simplified API for data operations.

4. **Storage Encryption Service** (`StorageEncryptionService`): Provides end-to-end encryption for stored data with federation-based access control for encryption keys.

5. **Versioning Manager** (`VersioningManager`): Manages data versioning, allowing tracking of changes to stored data over time with features for version history, retrieval, and rollback.

These components work together to enable secure, efficient data storage with federation-aware access control:

```
┌───────────────────┐     ┌───────────────────┐     ┌───────────────────┐
│ Federation 1      │     │ Federation 2      │     │ Federation 3      │
│                   │     │                   │     │                   │
│ ┌───────────────┐ │     │ ┌───────────────┐ │     │ ┌───────────────┐ │
│ │Storage Manager│ │     │ │Storage Manager│ │     │ │Storage Manager│ │
│ └───────┬───────┘ │     │ └───────┬───────┘ │     │ └───────┬───────┘ │
│         │         │     │         │         │     │         │         │
│ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │
│ │Storage Router │◄┼─────┼─┼─────►Storage Router ◄┼─┼─┼─────►Storage Router │
│ └───────┬───────┘ │     │ └───────┬───────┘ │     │ └───────┬───────┘ │
│         │         │     │         │         │     │         │         │
│ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │
│ │   Local DHT   │◄┼─────┼─┼─────►   Local DHT   ◄┼─┼─┼─────►   Local DHT   │
│ └───────┬───────┘ │     │ └───────┬───────┘ │     │ └───────┬───────┘ │
│         │         │     │         │         │     │         │         │
│ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │
│ │  Encryption   │ │     │ │  Encryption   │ │     │ │  Encryption   │ │
│ │    Service    │ │     │ │    Service    │ │     │ │    Service    │ │
│ └───────┬───────┘ │     │ └───────┬───────┘ │     │ └───────┬───────┘ │
│         │         │     │         │         │     │         │         │
│ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │     │ ┌───────┴───────┐ │
│ │  Versioning   │ │     │ │  Versioning   │ │     │ │  Versioning   │ │
│ │    Manager    │ │     │ │    Manager    │ │     │ │    Manager    │ │
│ └───────────────┘ │     │ └───────────────┘ │     │ └───────────────┘ │
└───────────────────┘     └───────────────────┘     └───────────────────┘
```

## Core Features

### 1. Proximity-Aware Peer Selection

The system selects storage peers based on multiple factors:
- Latency between nodes
- Available storage capacity
- Federation membership preference
- Uptime and reliability metrics

This ensures data is stored on the most suitable nodes for optimal access performance.

### 2. Federation-Based Access Control

Each piece of stored data can have fine-grained access policies that specify:
- Which federations can read the data
- Which federations can write/modify the data
- Which federations have admin (delete) privileges
- Encryption and redundancy requirements
- Versioning configuration

### 3. Cross-Federation Data Routing

The federation storage router enables:
- Data storage across multiple federations
- Automatic routing based on key prefixes
- Enforcing appropriate access controls
- Configurable replication strategies

### 4. Redundancy and Replication

Data is automatically replicated across multiple peers based on the specified redundancy factor, ensuring:
- Protection against node failures
- Better data availability
- Improved read performance through distributed access

### 5. End-to-End Encryption

The storage encryption service provides:
- AES-256-GCM encryption for sensitive data
- Federation-based access control for encryption keys
- Transparent encryption/decryption during storage operations
- Secure key management with granular access control

### 6. Data Versioning

The versioning manager enables:
- Tracking changes to stored data over time
- Maintaining a complete version history
- Accessing historical data versions
- Rolling back to previous versions
- Efficient storage of version data with appropriate access controls
- Configurable version retention policies

## Getting Started

### Basic Setup

1. Create a federation storage manager for your federation:

```rust
// Create required components
let local_storage = Arc::new(Storage::new("data/federation1"));
let dht = Arc::new(DistributedHashTable::new());
let federation_coordinator = Arc::new(FederationCoordinator::new());

// Configure your federation storage
let config = FederationStorageConfig {
    federation_id: "fed1".to_string(),
    max_storage_percentage: 0.8,
    auto_replication: true,
    default_redundancy_factor: 3,
    enable_cross_federation_storage: true,
    storage_namespace: "federation-data".to_string(),
};

// Create the storage manager
let storage_manager = FederationStorageManager::new(
    config,
    local_storage,
    dht,
    federation_coordinator,
    "node1".to_string(),
);
```

### Registering Storage Peers

Register local storage peers that will store data:

```rust
storage_manager.register_local_peer(
    "node1".to_string(),
    "192.168.1.1:8000".to_string(),
    1024 * 1024 * 1024, // 1GB capacity
    1024 * 1024 * 1024, // 1GB available
    HashMap::new(),
).await?;
```

### Storing and Retrieving Data

```rust
// Create an access policy
let mut policy = DataAccessPolicy::default();
policy.read_federations.insert("fed1".to_string());
policy.write_federations.insert("fed1".to_string());
policy.admin_federations.insert("fed1".to_string());

// Store data
let data = b"Hello, Federation Storage!";
storage_manager.store_data(
    "my/data/path.txt",
    data,
    Some(policy),
).await?;

// Retrieve data
let retrieved_data = storage_manager.retrieve_data("my/data/path.txt").await?;
```

### Cross-Federation Access

Configure routes for cross-federation storage:

```rust
storage_manager.configure_federation_route(
    "shared/".to_string(),
    vec!["fed1".to_string(), "fed2".to_string()],
    true,
    true,
    shared_policy,
).await?;
```

### Using End-to-End Encryption

To enable encryption for your data:

```rust
// 1. Initialize an encryption key with access for specific federations
let key_id = distributed_storage.initialize_encryption_key(
    vec!["fed1".to_string(), "fed2".to_string()]
).await?;

// 2. Create a policy that requires encryption
let mut policy = DataAccessPolicy::default();
policy.read_federations.insert("fed1".to_string());
policy.write_federations.insert("fed1".to_string());
policy.admin_federations.insert("fed1".to_string());
policy.encryption_required = true; // This flag enables encryption

// 3. Store data - it will be automatically encrypted
distributed_storage.put(
    "secure/data.txt",
    data,
    policy
).await?;

// 4. Retrieve data - it will be automatically decrypted
let decrypted_data = distributed_storage.get("secure/data.txt").await?;

// 5. Grant access to another federation
distributed_storage.grant_federation_key_access(
    "fed3".to_string(), 
    &key_id
).await?;
```

The encryption and decryption processes are handled transparently by the system, with these security guarantees:

- Data is encrypted using AES-256-GCM, a highly secure authenticated encryption algorithm
- Each piece of data uses a unique nonce to prevent replay attacks
- Encryption keys are managed per federation, allowing precise access control
- Key access can be granted or revoked for federations at any time
- Encryption metadata is stored alongside the data location for seamless retrieval

### Using Data Versioning

To enable and work with versioned data:

```rust
// 1. Create a policy with versioning enabled
let mut policy = DataAccessPolicy::default();
policy.read_federations.insert("fed1".to_string());
policy.write_federations.insert("fed1".to_string());
policy.admin_federations.insert("fed1".to_string());
policy.versioning_enabled = true; // Enable versioning
policy.max_versions = 10; // Keep up to 10 versions (optional)

// 2. Store data with versioning enabled - this creates the initial version
distributed_storage.put(
    "versioned/document.txt",
    initial_data,
    policy
).await?;

// 3. Update the data - this automatically creates a new version
distributed_storage.put(
    "versioned/document.txt",
    updated_data,
    policy
).await?;

// 4. List all versions for a key
let versions = distributed_storage.list_versions("versioned/document.txt").await?;
for version in versions {
    println!("Version: {}, Created: {}", version.version_id, version.created_at);
}

// 5. Retrieve a specific version by ID
let version_data = distributed_storage.get_version(
    "versioned/document.txt", 
    &version_id
).await?;

// 6. Revert to a previous version
distributed_storage.revert_to_version(
    "versioned/document.txt", 
    &old_version_id
).await?;

// 7. Enable versioning for existing unversioned data
distributed_storage.enable_versioning(
    "existing/document.txt", 
    5  // Keep up to 5 versions
).await?;
```

Versioning provides these benefits:

- Complete history of data changes
- Ability to retrieve and restore previous versions
- Protection against accidental data loss or corruption
- Audit trail of data modifications with timestamps and creator information
- Configurable retention policies to manage storage usage

## Federation Storage Statistics

Retrieve storage statistics for your federation:

```rust
let stats = storage_manager.get_federation_storage_stats().await?;
println!("Total capacity: {} bytes", stats.total_capacity);
println!("Available space: {} bytes", stats.available_space);
println!("Peer count: {}", stats.peer_count);
println!("Utilization: {}%", stats.utilization_percentage);
```

## Practical Examples

See the following files for practical examples:
- `src/bin/storage_demo.rs`: Demonstrates the distributed storage system with multiple federations
- `src/bin/encrypted_storage_demo.rs`: Shows how to use end-to-end encryption features
- `src/bin/versioned_storage_demo.rs`: Demonstrates data versioning capabilities
- `tests/federation_storage_tests.rs`: Integration tests showing various usage patterns

## Security Considerations

1. **Access Control**: Always set appropriate access policies for sensitive data.
2. **Encryption**: Enable encryption for sensitive data stored in the system.
3. **Federation Trust**: Only establish agreements with trusted federations.
4. **Key Management**: Be cautious when granting encryption key access to federations.
5. **Regular Monitoring**: Monitor storage statistics and peer health regularly.
6. **Versioning Policies**: Configure appropriate version retention policies to balance history preservation and storage usage.

## Future Enhancements

Future versions will include:
- Quantum-resistant cryptographic signatures
- AI-driven optimization of storage placement
- Enhanced cross-federation data migration capabilities
- Key rotation and versioning for long-term security
- Differential versioning to optimize storage efficiency
- Version tagging and semantic versioning support
- Version-aware replication and federation policies 