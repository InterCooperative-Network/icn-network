//! Network layer for the InterCooperative Network
//!
//! This crate provides networking capabilities for the InterCooperative Network,
//! including peer-to-peer networking, node discovery, messaging, and state synchronization.

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

/// Error types for network operations
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Error from libp2p networking
    #[error("Libp2p error: {0}")]
    Libp2pError(String),
    
    /// Error from the identity layer
    #[error("Identity error: {0}")]
    IdentityError(String),
    
    /// Error from the storage layer
    #[error("Storage error: {0}")]
    StorageError(String),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Deserialization error
    #[error("Deserialization error: {0}")]
    DeserializationError(String),
    
    /// Timeout error
    #[error("Network timeout")]
    Timeout,
    
    /// Peer not found
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    /// Protocol not supported
    #[error("Protocol not supported: {0}")]
    ProtocolNotSupported(String),
    
    /// Internal error
    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;

/// Message types that can be exchanged between nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum NetworkMessage {
    /// Identity announcement
    IdentityAnnouncement(IdentityAnnouncement),
    /// Transaction announcement
    TransactionAnnouncement(TransactionAnnouncement),
    /// Ledger state update
    LedgerStateUpdate(LedgerStateUpdate),
    /// Proposal announcement
    ProposalAnnouncement(ProposalAnnouncement),
    /// Vote announcement
    VoteAnnouncement(VoteAnnouncement),
    /// Custom message
    Custom(CustomMessage),
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

/// Custom message with flexible content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomMessage {
    /// Type of the custom message
    pub message_type: String,
    /// Custom data as a JSON object
    pub data: serde_json::Map<String, serde_json::Value>,
}

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: PeerId,
    /// Addresses where the peer can be reached
    pub addresses: Vec<Multiaddr>,
    /// Supported protocols
    pub protocols: Vec<String>,
    /// Whether the peer is currently connected
    pub connected: bool,
    /// Last seen timestamp (unix timestamp in seconds)
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

// Re-exports
pub use p2p::{P2pNetwork, P2pConfig};
pub use discovery::{PeerDiscovery, DiscoveryManager, DiscoveryConfig};
pub use messaging::{MessageProcessor, MessageEnvelope, DefaultMessageHandler};
pub use sync::{Synchronizer, SyncConfig, SyncState}; 