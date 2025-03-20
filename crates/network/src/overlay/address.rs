use serde::{Serialize, Deserialize};
use std::fmt;
use std::str::FromStr;
use std::net::Ipv6Addr;

/// Error for address-related operations
#[derive(Debug, Clone)]
pub enum AddressError {
    /// Invalid address format
    InvalidFormat(String),
    /// Address not available
    AddressNotAvailable(String),
    /// Address already allocated
    AddressAlreadyAllocated(String),
    /// Other error
    Other(String),
}

impl fmt::Display for AddressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressError::InvalidFormat(msg) => write!(f, "Invalid address format: {}", msg),
            AddressError::AddressNotAvailable(msg) => write!(f, "Address not available: {}", msg),
            AddressError::AddressAlreadyAllocated(msg) => write!(f, "Address already allocated: {}", msg),
            AddressError::Other(msg) => write!(f, "Other address error: {}", msg),
        }
    }
}

/// Overlay network address
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OverlayAddress(String);

impl OverlayAddress {
    /// Create a new overlay address
    pub fn new(address: String) -> Self {
        OverlayAddress(address)
    }
    
    /// Parse from a string
    pub fn from_string(address: &str) -> Result<Self, AddressError> {
        if address.is_empty() {
            return Err(AddressError::InvalidFormat("Empty address".to_string()));
        }
        Ok(OverlayAddress(address.to_string()))
    }
    
    /// Get the string representation
    pub fn as_string(&self) -> &str {
        &self.0
    }
    
    /// Try to convert to IPv6 address
    pub fn to_ipv6(&self) -> Result<Ipv6Addr, AddressError> {
        Ipv6Addr::from_str(&self.0)
            .map_err(|e| AddressError::InvalidFormat(format!("Invalid IPv6 address: {}", e)))
    }
}

impl fmt::Display for OverlayAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Address space for the overlay network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressSpace {
    /// Prefix (e.g., "fd00::/8")
    pub prefix: String,
    /// Name of the address space
    pub name: String,
    /// Description of the address space
    pub description: String,
}

/// Address allocation strategy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddressAllocationStrategy {
    /// Random allocation within the address space
    Random,
    /// Sequential allocation from the address space
    Sequential,
    /// Use a hash function to derive the address
    Hashed,
    /// Use a deterministic function based on node ID
    Deterministic,
} 