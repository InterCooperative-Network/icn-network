//! Overlay address management for the ICN network
//! 
//! This module provides types and functionality for allocating and managing
//! overlay network addresses.

use std::collections::HashMap;
use std::fmt;
use serde::{Serialize, Deserialize};

use crate::error::{Result, NetworkError};

/// Type of address space for the overlay network
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AddressSpace {
    /// IPv6-like address space
    Ipv6Like,
    /// Custom addressing scheme
    Custom,
}

/// Overlay address allocation strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AddressAllocationStrategy {
    /// Random address allocation
    Random,
    /// Address based on node ID
    NodeIdBased, 
    /// Address with federation prefix
    FederationPrefixed,
    /// Address based on geographic location
    GeographicBased,
}

/// An address in the overlay network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OverlayAddress {
    /// Address bytes (IPv6-like)
    pub bytes: [u8; 16],
    /// Federation ID if part of a federation
    pub federation: Option<String>,
}

impl fmt::Display for OverlayAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}:{:02x}{:02x}",
            self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3],
            self.bytes[4], self.bytes[5], self.bytes[6], self.bytes[7],
            self.bytes[8], self.bytes[9], self.bytes[10], self.bytes[11],
            self.bytes[12], self.bytes[13], self.bytes[14], self.bytes[15])
    }
}

/// Manages overlay address allocation
pub struct AddressAllocator {
    /// Type of address space
    address_space: AddressSpace,
    /// Map of allocated addresses
    allocated_addresses: HashMap<String, OverlayAddress>,
    /// Address allocation strategy
    allocation_strategy: AddressAllocationStrategy,
}

impl AddressAllocator {
    /// Create a new address allocator
    pub fn new() -> Self {
        Self {
            address_space: AddressSpace::Ipv6Like,
            allocated_addresses: HashMap::new(),
            allocation_strategy: AddressAllocationStrategy::FederationPrefixed,
        }
    }
    
    /// Allocate an overlay address
    pub fn allocate_address(&mut self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress> {
        // Check if already allocated
        if let Some(address) = self.allocated_addresses.get(node_id) {
            return Ok(address.clone());
        }
        
        // Generate address based on allocation strategy
        let address = match self.allocation_strategy {
            AddressAllocationStrategy::Random => {
                self.generate_random_address(federation_id)
            },
            AddressAllocationStrategy::NodeIdBased => {
                self.generate_node_id_based_address(node_id, federation_id)
            },
            AddressAllocationStrategy::FederationPrefixed => {
                self.generate_federation_prefixed_address(node_id, federation_id)
            },
            AddressAllocationStrategy::GeographicBased => {
                // Fall back to federation-prefixed for now
                self.generate_federation_prefixed_address(node_id, federation_id)
            }
        }?;
        
        // Store allocated address
        self.allocated_addresses.insert(node_id.to_string(), address.clone());
        
        Ok(address)
    }
    
    /// Generate a random overlay address
    fn generate_random_address(&self, federation_id: Option<&str>) -> Result<OverlayAddress> {
        let mut bytes = [0u8; 16];
        
        // Use crypto-secure randomness in real implementation
        for i in 0..16 {
            bytes[i] = ((i as u32 * 7) % 256) as u8;
        }
        
        Ok(OverlayAddress {
            bytes,
            federation: federation_id.map(|s| s.to_string()),
        })
    }
    
    /// Generate an address based on node ID
    fn generate_node_id_based_address(&self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress> {
        let mut bytes = [0u8; 16];
        
        // Hash the node ID
        let hash = calculate_hash(node_id.as_bytes());
        
        // Use first 16 bytes of hash
        for i in 0..16 {
            bytes[i] = hash[i % hash.len()];
        }
        
        Ok(OverlayAddress {
            bytes,
            federation: federation_id.map(|s| s.to_string()),
        })
    }
    
    /// Generate a federation-prefixed address
    fn generate_federation_prefixed_address(&self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress> {
        let mut bytes = [0u8; 16];
        
        // Use federation ID as prefix if available
        if let Some(fed_id) = federation_id {
            let fed_hash = calculate_hash(fed_id.as_bytes());
            
            // Use first 4 bytes as federation prefix
            for i in 0..4 {
                bytes[i] = fed_hash[i % fed_hash.len()];
            }
        }
        
        // Use hash of node ID for remaining bytes
        let hash = calculate_hash(node_id.as_bytes());
        
        // Copy hash bytes after federation prefix
        for i in 4..16 {
            bytes[i] = hash[(i - 4) % hash.len()];
        }
        
        Ok(OverlayAddress {
            bytes,
            federation: federation_id.map(|s| s.to_string()),
        })
    }
}

/// Calculate a SHA-256 hash of data
fn calculate_hash(data: &[u8]) -> [u8; 32] {
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}
