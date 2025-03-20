//! Distributed storage functionality for ICN
//!
//! This module provides distributed storage capabilities, including:
//! - Distributed Hash Table (DHT)
//! - Content Addressable Storage
//! - Data location and routing
//! - Storage policies
//! - Peer storage interaction
//! - Encryption

pub mod dht;
pub mod encryption;
pub mod location;
pub mod peer;
pub mod policy;
pub mod versioning;

// Re-exports
pub use dht::DistributedHashTable;
pub use encryption::StorageEncryption;
pub use location::StorageLocation;
pub use peer::StoragePeer;
pub use policy::StoragePolicy;
pub use versioning::DistributedVersioning;

/// Version of the distributed storage protocol
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// Default DHT port
pub const DEFAULT_DHT_PORT: u16 = 4000; 