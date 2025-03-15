//! Network layer for the InterCooperative Network
//!
//! This crate provides peer-to-peer networking capabilities for the ICN, including:
//! - Peer discovery and connection management
//! - Message serialization and exchange
//! - Network service interfaces
//! - Peer synchronization protocols
//! - Reputation management for peer reliability

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};
use thiserror::Error;
use serde::{Serialize, Deserialize};
use libp2p::PeerId;
use libp2p::Multiaddr;
use icn_core::storage::StorageError;

/// Error types for network operations
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Error in the network protocol
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
    
    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    /// Connection failed
    #[error("Connection failed")]
    ConnectionFailed,
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    
    /// Service stopped
    #[error("Service is stopped")]
    ServiceStopped,
    
    /// Service error
    #[error("Service error: {0}")]
    ServiceError(String),
    
    /// Service not enabled
    #[error("Service not enabled: {0}")]
    ServiceNotEnabled(String),
    
    /// Encoding error
    #[error("Encoding error")]
    EncodingError,
    
    /// Decoding error
    #[error("Decoding error")]
    DecodingError,
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
    
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Libp2p error
    #[error("Libp2p error: {0}")]
    Libp2pError(String),
    
    /// Error from the identity layer
    #[error("Identity error: {0}")]
    IdentityError(String),
    
    /// No relay servers available
    #[error("No relay servers available")]
    NoRelaysAvailable,
    
    /// Invalid relay address
    #[error("Invalid relay address")]
    InvalidRelayAddress,
    
    /// Relay connection error
    #[error("Relay connection error: {0}")]
    RelayConnectionError(String),
    
    /// Relay server error
    #[error("Relay server error: {0}")]
    RelayServerError(String),
    
    /// Maximum relay connections reached
    #[error("Maximum relay connections reached")]
    MaxRelayConnectionsReached,
}

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// Ledger state update
    #[serde(rename = "ledger.state")]
    LedgerStateUpdate(LedgerStateUpdate),
    
    /// Transaction announcement
    #[serde(rename = "ledger.transaction")]
    TransactionAnnouncement(TransactionAnnouncement),
    
    /// Identity announcement
    #[serde(rename = "identity.announcement")]
    IdentityAnnouncement(IdentityAnnouncement),
    
    /// Governance proposal announcement
    #[serde(rename = "governance.proposal")]
    ProposalAnnouncement(ProposalAnnouncement),
    
    /// Governance vote announcement
    #[serde(rename = "governance.vote")]
    VoteAnnouncement(VoteAnnouncement),
    
    /// Custom message type
    #[serde(rename = "custom")]
    CustomMessage(CustomMessage),
}

impl NetworkMessage {
    /// Get the message type as a string
    pub fn message_type(&self) -> String {
        match self {
            Self::LedgerStateUpdate(_) => "ledger.state".to_string(),
            Self::TransactionAnnouncement(_) => "ledger.transaction".to_string(),
            Self::IdentityAnnouncement(_) => "identity.announcement".to_string(),
            Self::ProposalAnnouncement(_) => "governance.proposal".to_string(),
            Self::VoteAnnouncement(_) => "governance.vote".to_string(),
            Self::CustomMessage(m) => m.message_type.clone(),
        }
    }
}

/// Identity announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentityAnnouncement {
    /// ID of the identity being announced
    pub identity_id: String,
    /// Public key of the identity
    pub public_key: Vec<u8>,
    /// Optional metadata for the identity
    pub metadata: HashMap<String, String>,
    /// Timestamp of the announcement
    pub timestamp: u64,
}

/// Transaction announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionAnnouncement {
    /// ID of the transaction
    pub transaction_id: String,
    /// Type of the transaction
    pub transaction_type: String,
    /// Timestamp of the transaction
    pub timestamp: u64,
    /// Sender of the transaction
    pub sender: String,
    /// Hash of the transaction data
    pub data_hash: String,
}

/// Ledger state update message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerStateUpdate {
    /// Hash of the ledger state
    pub ledger_hash: String,
    /// Transaction count
    pub transaction_count: u64,
    /// Account count
    pub account_count: u64,
    /// List of recent transaction IDs
    pub transaction_ids: Vec<String>,
    /// Timestamp of the update
    pub timestamp: u64,
}

/// Proposal announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalAnnouncement {
    /// ID of the proposal
    pub proposal_id: String,
    /// Title of the proposal
    pub title: String,
    /// Author of the proposal
    pub author: String,
    /// Timestamp of the proposal
    pub timestamp: u64,
    /// Voting end time
    pub voting_ends_at: u64,
    /// Hash of the proposal data
    pub data_hash: String,
}

/// Vote announcement message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteAnnouncement {
    /// ID of the proposal being voted on
    pub proposal_id: String,
    /// ID of the voter
    pub voter_id: String,
    /// Vote decision (yes, no, abstain)
    pub decision: String,
    /// Timestamp of the vote
    pub timestamp: u64,
    /// Hash of the vote data
    pub data_hash: String,
}

/// Custom message type for extensibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMessage {
    /// Message type
    pub message_type: String,
    /// Data for the message as JSON value
    pub data: serde_json::Map<String, serde_json::Value>,
}

/// Information about a peer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: String,
    /// Legacy ID field for compatibility
    pub id: String,
    /// Addresses the peer is listening on
    pub addresses: Vec<String>,
    /// Supported protocols
    pub protocols: Vec<String>,
    /// Whether the peer is currently connected
    pub connected: bool,
    /// Last seen timestamp
    pub last_seen: u64,
}

impl fmt::Display for PeerInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Peer {{ id: {}, connected: {}, protocols: {} }}",
               self.peer_id,
               self.connected,
               self.protocols.join(", "))
    }
}

/// Handler for received messages
#[async_trait]
pub trait MessageHandler: Send + Sync {
    /// Get the handler ID
    fn id(&self) -> usize;
    
    /// Get the handler name
    fn name(&self) -> &str;
    
    /// Handle a received message
    async fn handle_message(&self, message: &NetworkMessage, peer: &PeerInfo) -> NetworkResult<()>;
}

/// Network service interface
#[async_trait]
pub trait NetworkService: Send + Sync {
    /// Start the network service
    async fn start(&self) -> NetworkResult<()>;
    
    /// Stop the network service
    async fn stop(&self) -> NetworkResult<()>;
    
    /// Broadcast a message to all connected peers
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()>;
    
    /// Send a message to a specific peer
    async fn send_to(&self, peer_id: &PeerId, message: NetworkMessage) -> NetworkResult<()>;
    
    /// Connect to a peer
    async fn connect(&self, addr: &Multiaddr) -> NetworkResult<PeerId>;
    
    /// Disconnect from a peer
    async fn disconnect(&self, peer_id: &PeerId) -> NetworkResult<()>;
    
    /// Get information about a peer
    async fn get_peer_info(&self, peer_id: &PeerId) -> NetworkResult<PeerInfo>;
    
    /// Get a list of connected peers
    async fn get_connected_peers(&self) -> NetworkResult<Vec<PeerInfo>>;
    
    /// Register a handler for a specific message type
    async fn register_message_handler(&self, message_type: &str, handler: Arc<dyn MessageHandler>) -> NetworkResult<()>;
}

// Modules
pub mod p2p;
pub mod discovery;
pub mod messaging;
pub mod sync;
pub mod metrics;
pub mod reputation;
pub mod circuit_relay;

// Testing modules
#[cfg(test)]
mod tests;

// Public re-exports
pub use crate::p2p::{P2pConfig, P2pNetwork};
pub use crate::discovery::{DiscoveryConfig, DiscoveryService};
pub use crate::messaging::{MessageProcessor, PriorityConfig};
pub use crate::reputation::{ReputationConfig, ReputationManager};
pub use crate::circuit_relay::{CircuitRelayConfig, CircuitRelayManager};

// Re-export the messaging types for convenience
pub mod messages {
    pub use crate::messaging::IdentityAnnouncement;
    pub use crate::messaging::TransactionAnnouncement;
    pub use crate::messaging::LedgerStateUpdate;
    pub use crate::messaging::ProposalAnnouncement;
    pub use crate::messaging::VoteAnnouncement;
} 