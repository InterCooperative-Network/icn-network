//! Overlay network module

pub mod address;
pub mod dht;
pub mod routing;
pub mod onion;
pub mod tunneling;

// Re-export types
pub use self::address::{OverlayAddress, AddressSpace, AddressAllocationStrategy, AddressError};

pub use self::dht::{DistributedHashTable, Key, Value};

pub use self::routing::{RouteManager, RouteInfo, RoutingTable};

pub use self::tunneling::{TunnelManager, TunnelStatus, TunnelStats, TunnelError, WireGuardConfig};

pub use super::overlay::{
    OverlayNetworkManager, OverlayNetworkService, OverlayOptions,
    TunnelType, TunnelInfo, Ipv6Packet, ForwardingPolicy, MessagePriority
}; 