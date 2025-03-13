//! Networking functionality for the ICN network

mod node;
mod transport;
mod overlay;

pub use node::{Node, NodeId, NodeInfo, NodeStatus};
pub use transport::{Transport, TransportConfig, SecurityLevel};
pub use overlay::{
    OverlayNetworkManager, OverlayNetworkService, OverlayAddress, 
    OverlayOptions, MessagePriority,
};

// Re-export error types
pub use crate::error::{Result, NetworkError};
