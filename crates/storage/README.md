# ICN Storage

This crate provides the storage system for the InterCooperative Network (ICN).

## Overview

The storage crate is responsible for data storage and management in the ICN system, including:

- Local storage backends (memory, file-based)
- Data versioning and history
- Storage quotas and resource management
- Metrics and monitoring
- Distributed storage capabilities
- Federation-based storage routing

## Structure

The crate is organized into the following modules:

- `memory_storage`: In-memory storage implementation
- `metrics`: Storage metrics collection and reporting
- `quota`: Storage resource management
- `versioning`: Data versioning and history tracking
- `distributed`: Distributed storage implementation
  - `dht`: Distributed hash table
  - `encryption`: Data encryption
  - `location`: Storage location management
  - `peer`: Peer storage interaction
  - `policy`: Storage policies
  - `versioning`: Distributed versioning
- `federation`: Federation-based storage routing
  - `router`: Storage router
  - `strategies`: Routing strategies

## Features

This crate supports the following feature flags:

- `distributed`: Enables distributed storage capabilities (enabled by default)
- `federation`: Enables federation-based storage routing (enabled by default)

## Consolidated Modules

This crate has consolidated functionality from the following previously separate crates:

- `distributed-storage`: Now incorporated as `storage::distributed`
- `federation-storage-router`: Now incorporated as `storage::federation`

## Usage

To use this crate, add it to your `Cargo.toml`:

```toml
[dependencies]
icn-storage = { path = "../storage" }
```

Basic usage example:

```rust
use icn_storage::{StorageBackend, MemoryStorage, VersionedStorage};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a memory-based storage backend
    let storage = MemoryStorage::new();
    
    // Create a versioned storage wrapper
    let versioned_storage = VersionedStorage::new(storage);
    
    // Store data
    versioned_storage.store("key1", b"value1".to_vec())?;
    
    // Retrieve data
    let data = versioned_storage.get("key1")?;
    println!("Retrieved data: {:?}", data);
    
    // For distributed storage
    #[cfg(feature = "distributed")]
    {
        use icn_storage::distributed::{DistributedHashTable, StoragePolicy};
        
        // Create a DHT
        let dht = DistributedHashTable::new();
        
        // Set up storage policy
        let policy = StoragePolicy::default();
        
        // Use DHT with policy
        // ...
    }
    
    Ok(())
} 