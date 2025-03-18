// Compatibility layer for libp2p version differences

// Import libp2p transport types
use libp2p::core::transport::PortUse as LibP2pPortUse;

/// PortUse compatibility enum to handle version mismatches
pub enum PortUse {
    /// No specific port used
    NoPortUse,
    /// Used for making a new port
    New,
    /// Used for reusing a port
    Reuse
}

/// Default implementation for PortUse
impl Default for PortUse {
    fn default() -> Self {
        Self::NoPortUse
    }
}

impl From<LibP2pPortUse> for PortUse {
    fn from(value: LibP2pPortUse) -> Self {
        match value {
            LibP2pPortUse::New => Self::New,
            LibP2pPortUse::Reuse => Self::Reuse,
        }
    }
}

impl From<PortUse> for LibP2pPortUse {
    fn from(value: PortUse) -> Self {
        match value {
            PortUse::NoPortUse => LibP2pPortUse::New, // Default to New for NoPortUse
            PortUse::New => LibP2pPortUse::New,
            PortUse::Reuse => LibP2pPortUse::Reuse,
        }
    }
} 