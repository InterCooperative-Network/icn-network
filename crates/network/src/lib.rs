//! Network layer for the InterCooperative Network
//!
//! This crate provides peer-to-peer networking capabilities for the ICN, including:
//! - Peer discovery and connection management
//! - Message serialization and exchange
//! - Network service interfaces
//! - Peer synchronization protocols
//! - Reputation management for peer reliability
//! - Overlay network functionality for advanced routing

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use thiserror::Error;
use serde::{Serialize, Deserialize};
use libp2p::PeerId;
use libp2p::Multiaddr;
use icn_core::storage::StorageError;

/// Network error types
#[derive(Debug, Error, Clone)]
pub enum NetworkError {
    /// Storage error
    #[error("Storage error: {0}")]
    StorageError(#[from] StorageError),
    
    /// Encoding error
    #[error("Encoding error")]
    EncodingError,
    
    /// Decoding error
    #[error("Decoding error")]
    DecodingError,
    
    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    /// Connection error
    #[error("Connection error: {0}")]
    ConnectionError(String),
    
    /// Message error
    #[error("Message error: {0}")]
    MessageError(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
    
    /// Channel closed
    #[error("Channel closed: {0}")]
    ChannelClosed(String),
    
    /// Other error
    #[error("Other error: {0}")]
    Other(String),
    
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

    /// Service stopped
    #[error("Service stopped")]
    ServiceStopped,
    
    /// Service error
    #[error("Service error: {0}")]
    ServiceError(String),
    
    /// Service not enabled
    #[error("Service not enabled: {0}")]
    ServiceNotEnabled(String),
    
    /// Connection failed
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    /// Invalid address
    #[error("Invalid address: {0}")]
    InvalidAddress(String),
    
    /// Queue is full
    #[error("Queue is full")]
    QueueFull,
    
    /// Invalid priority
    #[error("Invalid message priority")]
    InvalidPriority,
    
    /// No connections available
    #[error("No connections available")]
    NoConnectionsAvailable,
    
    /// Transport error
    #[error("Transport error: {0}")]
    TransportError(String),
    
    /// Invalid peer ID
    #[error("Invalid peer ID: {0}")]
    InvalidPeerId(String),
    
    /// Reputation system error
    #[error("Reputation error: {0}")]
    ReputationError(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
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
    Custom(CustomMessage),
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
            Self::Custom(m) => m.message_type.clone(),
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
    /// Unique identifier for the peer
    pub id: String,
    /// Peer ID
    pub peer_id: String,
    /// Addresses the peer can be reached at
    pub addresses: Vec<String>,
    /// Protocol versions supported by the peer
    pub protocols: Vec<String>,
    /// Agent version string
    pub agent_version: Option<String>,
    /// Protocol version
    pub protocol_version: Option<String>,
    /// Whether the peer is currently connected
    pub connected: bool,
    /// Last seen timestamp
    pub last_seen: Option<u64>,
    /// Reputation score
    pub reputation: Option<i32>,
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

/// Network service trait
#[async_trait]
pub trait NetworkService: Send + Sync + 'static {
    /// Start the network service
    async fn start(&self) -> NetworkResult<()>;
    
    /// Stop the network service
    async fn stop(&self) -> NetworkResult<()>;
    
    /// Broadcast a message to all connected peers
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()>;
    
    /// Send a message to a specific peer
    async fn send_to(&self, peer_id: &str, message: NetworkMessage) -> NetworkResult<()>;
    
    /// Connect to a peer
    async fn connect(&self, address: Multiaddr) -> NetworkResult<PeerId>;
    
    /// Disconnect from a peer
    async fn disconnect(&self, peer_id: &str) -> NetworkResult<()>;
    
    /// Get information about a peer
    async fn get_peer_info(&self, peer_id: &str) -> NetworkResult<PeerInfo>;
    
    /// Get a list of connected peers
    async fn get_connected_peers(&self) -> NetworkResult<Vec<PeerInfo>>;
    
    /// Register a handler for a specific message type
    async fn register_message_handler(&self, message_type: &str, handler: Arc<dyn MessageHandler>) -> NetworkResult<()>;
    
    /// Subscribe to receive network messages
    /// Returns a channel receiver that will receive (peer_id, message) tuples
    async fn subscribe_messages(&self) -> NetworkResult<mpsc::Receiver<(String, NetworkMessage)>>;
}

/// Public modules
pub mod p2p;
pub mod discovery;
pub mod messaging;
pub mod sync;
pub mod metrics;
pub mod reputation;
pub mod reputation_system;
pub mod circuit_relay;
pub mod adapter;
pub mod overlay;
pub mod resource_sharing;

/// Private modules
mod libp2p_compat;
mod config;
mod test_reputation;
mod tests;

/// Re-exports
pub use crate::p2p::{P2pConfig, P2pNetwork};
pub use crate::discovery::DiscoveryConfig;
pub use crate::messaging::{MessageProcessor, PriorityConfig};
pub use crate::reputation::{ReputationConfig, ReputationManager, ReputationChange};
pub use crate::circuit_relay::{CircuitRelayConfig, CircuitRelayManager};

/// Re-export the messaging types for convenience
pub mod messages {
    pub use crate::{
        IdentityAnnouncement,
        TransactionAnnouncement,
        LedgerStateUpdate,
        ProposalAnnouncement,
        VoteAnnouncement,
        CustomMessage,
    };
}

/// Serialization helpers for PeerId
mod peer_id_serde {
    use libp2p::PeerId;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::str::FromStr;

    pub fn serialize<S>(peer_id: &PeerId, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = peer_id.to_string();
        s.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<PeerId, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        PeerId::from_str(&s).map_err(serde::de::Error::custom)
    }
}

/// Re-export adapter functions
pub use adapter::{core_to_network_message, network_to_core_message};

/// Re-export overlay functionality
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

/// Re-export node types
pub use overlay::node::{Node, NodeId, NodeInfo, NodeStatus};

/// Re-export tunneling functionality
pub use overlay::tunneling::{
    TunnelManager, TunnelStats, TunnelStatus, TunnelError, WireGuardConfig
};

/// Re-export reputation types for backward compatibility
pub use reputation_system::{
    ReputationSystem,
    AttestationType,
    Evidence,
    Attestation,
    TrustScore,
    SybilIndicators,
    ReputationError,
}; 