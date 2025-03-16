//! Overlay networking module for ICN
//!
//! This module provides components for implementing an overlay network
//! using IPv6 addresses and tunneling.

pub mod address;
pub mod routing;
pub mod dht;
pub mod onion;
pub mod tunneling;

pub use address::{OverlayAddress, AddressAllocator, AddressSpace, AddressAllocationStrategy, AddressError};
pub use routing::{RouteManager, RouteInfo, RoutingTable};
pub use dht::{DistributedHashTable, NodeInfo, Key, Value};
pub use onion::{OnionRouter, Circuit};
pub use tunneling::{TunnelManager, TunnelStats, TunnelStatus, TunnelError, WireGuardConfig}; 