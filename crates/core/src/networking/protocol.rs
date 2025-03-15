//! Protocol definitions for network communication
//!
//! This module defines the protocol specifications for communication
//! between nodes in the network.

use std::fmt;
use serde::{Serialize, Deserialize};
use super::NetworkError;

/// Protocol version
pub const PROTOCOL_VERSION: &str = "0.1.0";

/// Message types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    /// Handshake message for connection establishment
    Handshake,
    /// Generic data transfer
    Data,
    /// Keep-alive ping
    Ping,
    /// Response to a ping
    Pong,
    /// Request for resources or information
    Request,
    /// Response to a request
    Response,
    /// Broadcast message to multiple peers
    Broadcast,
    /// Notification of an event
    Notification,
    /// Error message
    Error,
}

impl Default for MessageType {
    fn default() -> Self {
        Self::Data
    }
}

/// A protocol message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolMessage {
    /// Protocol version
    pub version: String,
    /// Message type
    pub message_type: MessageType,
    /// Message ID for correlation
    pub message_id: String,
    /// Sender node ID
    pub sender: String,
    /// Recipient node ID (optional for broadcasts)
    pub recipient: Option<String>,
    /// Message payload
    pub payload: Vec<u8>,
    /// Timestamp in milliseconds
    pub timestamp: u64,
    /// Time-to-live for relayed messages
    pub ttl: Option<u32>,
    /// Headers for additional metadata
    pub headers: Vec<MessageHeader>,
}

/// A protocol message header
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageHeader {
    /// Header name
    pub name: String,
    /// Header value
    pub value: String,
}

impl ProtocolMessage {
    /// Create a new protocol message
    pub fn new(
        message_type: MessageType,
        message_id: String,
        sender: String,
        recipient: Option<String>,
        payload: Vec<u8>,
    ) -> Self {
        // Get current timestamp in milliseconds
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        
        Self {
            version: PROTOCOL_VERSION.to_string(),
            message_type,
            message_id,
            sender,
            recipient,
            payload,
            timestamp,
            ttl: None,
            headers: Vec::new(),
        }
    }
    
    /// Add a header to the message
    pub fn add_header(&mut self, name: &str, value: &str) {
        self.headers.push(MessageHeader {
            name: name.to_string(),
            value: value.to_string(),
        });
    }
    
    /// Find a header by name
    pub fn get_header(&self, name: &str) -> Option<&str> {
        self.headers.iter()
            .find(|h| h.name == name)
            .map(|h| h.value.as_str())
    }
    
    /// Convert the message to bytes
    pub fn to_bytes(&self) -> Result<Vec<u8>, NetworkError> {
        bincode::serialize(self)
            .map_err(|e| NetworkError::SerializationError(format!("Failed to serialize message: {}", e)))
    }
    
    /// Create a message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, NetworkError> {
        bincode::deserialize(bytes)
            .map_err(|e| NetworkError::SerializationError(format!("Failed to deserialize message: {}", e)))
    }
} 