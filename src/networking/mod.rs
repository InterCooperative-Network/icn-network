//! Networking functionality for the ICN network

mod node;
pub mod overlay;

pub use node::{Node, NodeId, NodeInfo, NodeStatus};
pub use overlay::{
    // Core overlay components
    OverlayNetworkManager, OverlayNetworkService, OverlayAddress, 
    OverlayOptions, MessagePriority, Ipv6Packet,
    
    // Tunnel-related functionality
    TunnelType, TunnelInfo, ForwardingPolicy,
    
    // Address components from overlay::address
    AddressSpace, AddressAllocationStrategy, AddressError,
    
    // DHT components
    DistributedHashTable, Key, Value,
};

// Tunneling functionality
pub use overlay::tunneling::{
    TunnelManager, TunnelStats, TunnelStatus, TunnelError, WireGuardConfig
};

// Re-export error types
pub use crate::error::{Result, NetworkError};
