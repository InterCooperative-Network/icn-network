//! Overlay address management for the ICN network
//! 
//! This module provides types and functionality for allocating and managing
//! overlay network addresses based on IPv6.

use std::collections::HashMap;
use std::fmt;
use std::net::{IpAddr, Ipv6Addr};
use std::str::FromStr;
use serde::{Serialize, Deserialize};
use thiserror::Error;

use crate::error::{Result, NetworkError};

/// Errors related to overlay addressing
#[derive(Error, Debug)]
pub enum AddressError {
    #[error("Invalid IPv6 address format: {0}")]
    InvalidFormat(String),
    #[error("Address already in use")]
    AddressInUse,
    #[error("Federation prefix not available")]
    FederationPrefixNotAvailable,
    #[error("Address space exhausted")]
    AddressSpaceExhausted,
}

/// Type of address space for the overlay network
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AddressSpace {
    /// IPv6-like address space
    Ipv6Like,
    /// ULA (Unique Local Address) IPv6 space (fc00::/7)
    UniqueLocal,
    /// GUA (Global Unicast Address) IPv6 space (2000::/3)
    GlobalUnicast,
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
    /// Subnet prefix length (e.g., /64)
    pub prefix_len: u8,
}

impl OverlayAddress {
    /// Create a new overlay address from an IPv6 address
    pub fn from_ipv6(addr: Ipv6Addr, federation: Option<String>, prefix_len: u8) -> Self {
        Self {
            bytes: addr.octets(),
            federation,
            prefix_len,
        }
    }
    
    /// Convert to an IPv6 address
    pub fn to_ipv6(&self) -> Ipv6Addr {
        Ipv6Addr::from(self.bytes)
    }
    
    /// Check if this address is in the same subnet as another address
    pub fn in_same_subnet(&self, other: &Self) -> bool {
        if self.prefix_len != other.prefix_len {
            return false;
        }
        
        // Calculate how many full bytes to compare
        let full_bytes = (self.prefix_len / 8) as usize;
        
        // First check all the full bytes
        if self.bytes[..full_bytes] != other.bytes[..full_bytes] {
            return false;
        }
        
        // If prefix length is not a multiple of 8, check the partial byte
        let remaining_bits = self.prefix_len % 8;
        if remaining_bits > 0 {
            let mask = 0xff_u8 << (8 - remaining_bits);
            return (self.bytes[full_bytes] & mask) == (other.bytes[full_bytes] & mask);
        }
        
        true
    }
    
    /// Get the subnet prefix as a new OverlayAddress
    pub fn get_subnet_prefix(&self) -> Self {
        let mut bytes = self.bytes;
        let full_bytes = (self.prefix_len / 8) as usize;
        let remaining_bits = self.prefix_len % 8;
        
        // Zero out all bytes after the prefix
        for i in full_bytes + 1..16 {
            bytes[i] = 0;
        }
        
        // Handle the partial byte
        if remaining_bits > 0 {
            let mask = 0xff_u8 << (8 - remaining_bits);
            bytes[full_bytes] &= mask;
        }
        
        Self {
            bytes,
            federation: self.federation.clone(),
            prefix_len: self.prefix_len,
        }
    }
    
    /// Check if this is a unique local address (ULA)
    pub fn is_unique_local(&self) -> bool {
        (self.bytes[0] & 0xfe) == 0xfc // fc00::/7
    }
    
    /// Check if this is a global unicast address
    pub fn is_global_unicast(&self) -> bool {
        (self.bytes[0] & 0xe0) == 0x20 // 2000::/3
    }
}

impl fmt::Display for OverlayAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Format as standard IPv6 address with prefix length
        let ipv6 = self.to_ipv6();
        write!(f, "{}/{}", ipv6, self.prefix_len)
    }
}

impl FromStr for OverlayAddress {
    type Err = AddressError;
    
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        // Parse IPv6 CIDR notation (e.g., "2001:db8::1/64")
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(AddressError::InvalidFormat(
                "Address must be in CIDR notation (IPv6/prefix)".to_string()
            ));
        }
        
        let addr = Ipv6Addr::from_str(parts[0])
            .map_err(|_| AddressError::InvalidFormat(format!("Invalid IPv6 address: {}", parts[0])))?;
            
        let prefix_len = parts[1].parse::<u8>()
            .map_err(|_| AddressError::InvalidFormat(format!("Invalid prefix length: {}", parts[1])))?;
            
        if prefix_len > 128 {
            return Err(AddressError::InvalidFormat(
                format!("Prefix length must be <= 128, got {}", prefix_len)
            ));
        }
        
        Ok(Self {
            bytes: addr.octets(),
            federation: None,
            prefix_len,
        })
    }
}

/// Manages overlay address allocation
pub struct AddressAllocator {
    /// Type of address space
    address_space: AddressSpace,
    /// Map of allocated addresses
    allocated_addresses: HashMap<String, OverlayAddress>,
    /// Map of federation prefixes
    federation_prefixes: HashMap<String, OverlayAddress>,
    /// Address allocation strategy
    allocation_strategy: AddressAllocationStrategy,
    /// Federation base prefix length (e.g., /48 for federations)
    federation_prefix_len: u8,
    /// Node prefix length (e.g., /64 for nodes)
    node_prefix_len: u8,
}

impl AddressAllocator {
    /// Create a new address allocator
    pub fn new() -> Self {
        Self {
            address_space: AddressSpace::UniqueLocal,
            allocated_addresses: HashMap::new(),
            federation_prefixes: HashMap::new(),
            allocation_strategy: AddressAllocationStrategy::FederationPrefixed,
            federation_prefix_len: 48,
            node_prefix_len: 64,
        }
    }
    
    /// Create a new address allocator with custom settings
    pub fn with_settings(
        address_space: AddressSpace,
        allocation_strategy: AddressAllocationStrategy,
        federation_prefix_len: u8,
        node_prefix_len: u8,
    ) -> Self {
        Self {
            address_space,
            allocated_addresses: HashMap::new(),
            federation_prefixes: HashMap::new(),
            allocation_strategy,
            federation_prefix_len,
            node_prefix_len,
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
                self.generate_random_address(federation_id)?
            },
            AddressAllocationStrategy::NodeIdBased => {
                self.generate_node_id_based_address(node_id, federation_id)?
            },
            AddressAllocationStrategy::FederationPrefixed => {
                self.generate_federation_prefixed_address(node_id, federation_id)?
            },
            AddressAllocationStrategy::GeographicBased => {
                // Fall back to federation-prefixed for now
                self.generate_federation_prefixed_address(node_id, federation_id)?
            }
        };
        
        // Store allocated address
        self.allocated_addresses.insert(node_id.to_string(), address.clone());
        
        Ok(address)
    }
    
    /// Allocate a federation prefix
    pub fn allocate_federation_prefix(&mut self, federation_id: &str) -> Result<OverlayAddress> {
        // Check if already allocated
        if let Some(prefix) = self.federation_prefixes.get(federation_id) {
            return Ok(prefix.clone());
        }
        
        // Start with a base prefix depending on address space
        let mut prefix_bytes = [0u8; 16];
        match self.address_space {
            AddressSpace::UniqueLocal => {
                // Use fc00::/7 (ULA)
                prefix_bytes[0] = 0xfd; // Use fd00::/8 from ULA space for deterministic allocation
            },
            AddressSpace::GlobalUnicast => {
                // Use 2001:db8::/32 (Documentation prefix) - in a real system, this would be a GUA
                prefix_bytes[0] = 0x20;
                prefix_bytes[1] = 0x01;
                prefix_bytes[2] = 0x0d;
                prefix_bytes[3] = 0xb8;
            },
            _ => {
                // For other types, use ULA
                prefix_bytes[0] = 0xfd;
            }
        }
        
        // Use hash of federation ID for the next bytes
        let hash = calculate_hash(federation_id.as_bytes());
        
        // Determine how many bytes to use for federation ID (based on prefix length)
        let fed_bytes = ((self.federation_prefix_len + 7) / 8) as usize;
        
        // Apply the hash to the relevant portion of the address
        // We start at byte 1 for ULA or byte 4 for GUA
        let start_byte = if self.address_space == AddressSpace::GlobalUnicast { 4 } else { 1 };
        
        for i in 0..(fed_bytes - start_byte) {
            if start_byte + i < 16 {
                prefix_bytes[start_byte + i] = hash[i % hash.len()];
            }
        }
        
        // Ensure bytes after prefix are zero
        for i in ((self.federation_prefix_len + 7) / 8) as usize..16 {
            prefix_bytes[i] = 0;
        }
        
        // Create the federation prefix
        let prefix = OverlayAddress {
            bytes: prefix_bytes,
            federation: Some(federation_id.to_string()),
            prefix_len: self.federation_prefix_len,
        };
        
        // Store allocated prefix
        self.federation_prefixes.insert(federation_id.to_string(), prefix.clone());
        
        Ok(prefix)
    }
    
    /// Generate a random overlay address
    fn generate_random_address(&self, federation_id: Option<&str>) -> Result<OverlayAddress> {
        let mut bytes = [0u8; 16];
        
        // Start with appropriate prefix based on address space
        match self.address_space {
            AddressSpace::UniqueLocal => {
                // Use fc00::/7 (ULA)
                bytes[0] = 0xfd; // Use fd00::/8 from ULA space
            },
            AddressSpace::GlobalUnicast => {
                // Use 2001::/16 (Global Unicast)
                bytes[0] = 0x20;
                bytes[1] = 0x01;
            },
            _ => {
                // For other types, use ULA
                bytes[0] = 0xfd;
            }
        }
        
        // Use crypto-secure randomness for the rest
        // In a real implementation, this would use a proper RNG
        for i in 2..16 {
            bytes[i] = ((i as u32 * 13 + 7) % 256) as u8;
        }
        
        Ok(OverlayAddress {
            bytes,
            federation: federation_id.map(|s| s.to_string()),
            prefix_len: self.node_prefix_len,
        })
    }
    
    /// Generate an address based on node ID
    fn generate_node_id_based_address(&self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress> {
        let mut bytes = [0u8; 16];
        
        // Start with appropriate prefix based on address space
        match self.address_space {
            AddressSpace::UniqueLocal => {
                // Use fc00::/7 (ULA)
                bytes[0] = 0xfd; // Use fd00::/8 from ULA space
            },
            AddressSpace::GlobalUnicast => {
                // Use 2001::/16 (Global Unicast)
                bytes[0] = 0x20;
                bytes[1] = 0x01;
            },
            _ => {
                // For other types, use ULA
                bytes[0] = 0xfd;
            }
        }
        
        // Hash the node ID
        let hash = calculate_hash(node_id.as_bytes());
        
        // Use hash for host part (last 8 bytes for /64)
        let start_byte = 16 - ((128 - self.node_prefix_len) / 8) as usize;
        for i in start_byte..16 {
            bytes[i] = hash[(i - start_byte) % hash.len()];
        }
        
        Ok(OverlayAddress {
            bytes,
            federation: federation_id.map(|s| s.to_string()),
            prefix_len: self.node_prefix_len,
        })
    }
    
    /// Generate a federation-prefixed address
    fn generate_federation_prefixed_address(&self, node_id: &str, federation_id: Option<&str>) -> Result<OverlayAddress> {
        let mut bytes = [0u8; 16];
        
        // If federation ID is provided, use its prefix
        if let Some(fed_id) = federation_id {
            // Get or allocate federation prefix
            let fed_prefix = self.allocate_federation_prefix(fed_id)?;
            
            // Copy federation prefix
            bytes.copy_from_slice(&fed_prefix.bytes);
            
            // Hash the node ID
            let hash = calculate_hash(node_id.as_bytes());
            
            // Use hash for host part of address
            // Calculate which byte to start the host part
            let prefix_bytes = ((self.federation_prefix_len + 7) / 8) as usize;
            let host_bytes = ((self.node_prefix_len - self.federation_prefix_len) / 8) as usize;
            
            // Fill subnet bytes if any
            for i in 0..host_bytes {
                if prefix_bytes + i < 16 {
                    bytes[prefix_bytes + i] = hash[i % hash.len()];
                }
            }
            
            // Use remaining hash bytes for interface identifier
            let iface_start = ((self.node_prefix_len + 7) / 8) as usize;
            for i in iface_start..16 {
                bytes[i] = hash[(i - iface_start + host_bytes) % hash.len()];
            }
            
            // Set Modified EUI-64 bits according to RFC 4291
            // (though we don't strictly follow EUI-64 here)
            bytes[8] = (bytes[8] & 0xfe) | 0x02; // Set universal/local bit
            
            Ok(OverlayAddress {
                bytes,
                federation: Some(fed_id.to_string()),
                prefix_len: self.node_prefix_len,
            })
        } else {
            // No federation, generate a node-based address
            self.generate_node_id_based_address(node_id, None)
        }
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
