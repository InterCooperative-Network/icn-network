//! Networking module for ICN
//!
//! This module provides peer-to-peer networking capabilities for nodes in the 
//! InterCooperative Network.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use thiserror::Error;
use tracing::{debug, error, info, warn};
use async_trait::async_trait;

/// Error types for networking operations
#[derive(Error, Debug)]
pub enum NetworkError {
    /// Error connecting to a peer
    #[error("Failed to connect to peer: {0}")]
    ConnectionError(String),
    
    /// Error sending message
    #[error("Failed to send message: {0}")]
    SendError(String),
    
    /// Error receiving message
    #[error("Failed to receive message: {0}")]
    ReceiveError(String),
    
    /// IO Error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    /// Timeout error
    #[error("Operation timed out")]
    Timeout,
}

/// Result type for network operations
pub type NetworkResult<T> = Result<T, NetworkError>;

/// A message that can be sent over the network
#[derive(Debug, Clone)]
pub struct NetworkMessage {
    /// The message type
    pub message_type: String,
    /// The message payload
    pub payload: Vec<u8>,
    /// The sender's node ID
    pub sender: String,
    /// The intended recipient's node ID, if any
    pub recipient: Option<String>,
    /// Timestamp when the message was created
    pub timestamp: u64,
}

/// The core network interface
#[async_trait]
pub trait Network: Send + Sync {
    /// Start the networking service
    async fn start(&self) -> NetworkResult<()>;
    
    /// Stop the networking service
    async fn stop(&self) -> NetworkResult<()>;
    
    /// Connect to a peer
    async fn connect(&self, address: SocketAddr) -> NetworkResult<()>;
    
    /// Disconnect from a peer
    async fn disconnect(&self, peer_id: &str) -> NetworkResult<()>;
    
    /// Send a message to a specific peer
    async fn send_to(&self, peer_id: &str, message: NetworkMessage) -> NetworkResult<()>;
    
    /// Broadcast a message to all connected peers
    async fn broadcast(&self, message: NetworkMessage) -> NetworkResult<()>;
    
    /// Register a message handler
    async fn register_handler<F>(&self, message_type: &str, handler: F) -> NetworkResult<()>
    where
        F: Fn(NetworkMessage) -> NetworkResult<()> + Send + Sync + 'static;
    
    /// Get a list of connected peers
    async fn get_peers(&self) -> NetworkResult<Vec<String>>;
}

pub mod tcp;
pub mod discovery;
pub mod protocol;

// Re-exports
pub use tcp::TcpNetwork; 